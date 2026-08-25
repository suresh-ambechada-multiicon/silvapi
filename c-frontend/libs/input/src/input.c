#include "input/input.h"

#define INPUT_MAX_EVENTS 256
#define INPUT_TEXT_CAP   256

typedef struct {
    char        name[48];
    int         key;
    uint32_t    mods;
    Rect        area;
    InputAction fn;
    void*       user;
    bool        is_click;
    bool        used;
} Binding;

static struct {
    InputEvent events[INPUT_MAX_EVENTS];
    size_t     event_count;

    Vec2  mouse, mouse_prev, scroll;
    bool  down[INPUT_MOUSE_COUNT];
    bool  pressed[INPUT_MOUSE_COUNT];
    bool  released[INPUT_MOUSE_COUNT];
    Vec2  press_origin[INPUT_MOUSE_COUNT];

    bool  key_down[INPUT_KEY_COUNT];
    bool  key_pressed[INPUT_KEY_COUNT];
    bool  key_released[INPUT_KEY_COUNT];
    uint32_t mods;

    char   text[INPUT_TEXT_CAP];
    size_t text_len;

    int    click_count;
    double last_click_time;
    Vec2   last_click_pos;

    double click_window;
    float  click_slop;
    double now;

    Binding  bindings[INPUT_MAX_BINDINGS];
    bool     first_frame;
} S;

void input_init(void) {
    memset(&S, 0, sizeof(S));
    S.click_window = 0.35;
    S.click_slop = 6.0f;
    S.first_frame = true;
}

void input_set_click_thresholds(double seconds, float distance_px) {
    if (seconds > 0.0)      S.click_window = seconds;
    if (distance_px > 0.0f) S.click_slop = distance_px;
}

void input_begin_frame(double now) {
    S.now = now;
    S.event_count = 0;

    S.mouse_prev = S.mouse;
    // On the very first frame there is no previous position, so a delta would
    // be the whole distance from the origin.
    if (S.first_frame) {
        S.mouse_prev = S.mouse;
        S.first_frame = false;
    }

    S.scroll = (Vec2){0, 0};
    memset(S.pressed, 0, sizeof(S.pressed));
    memset(S.released, 0, sizeof(S.released));
    memset(S.key_pressed, 0, sizeof(S.key_pressed));
    memset(S.key_released, 0, sizeof(S.key_released));
    S.text[0] = '\0';
    S.text_len = 0;

    // Click regions come from layout, which moves every frame; key bindings are
    // registered once and persist.
    for (size_t i = 0; i < INPUT_MAX_BINDINGS; i++) {
        if (S.bindings[i].used && S.bindings[i].is_click) S.bindings[i].used = false;
    }
}

static void append_utf8(uint32_t cp) {
    char buf[4];
    size_t n = 0;
    if (cp < 0x80) {
        buf[n++] = (char)cp;
    } else if (cp < 0x800) {
        buf[n++] = (char)(0xC0 | (cp >> 6));
        buf[n++] = (char)(0x80 | (cp & 0x3F));
    } else if (cp < 0x10000) {
        buf[n++] = (char)(0xE0 | (cp >> 12));
        buf[n++] = (char)(0x80 | ((cp >> 6) & 0x3F));
        buf[n++] = (char)(0x80 | (cp & 0x3F));
    } else {
        buf[n++] = (char)(0xF0 | (cp >> 18));
        buf[n++] = (char)(0x80 | ((cp >> 12) & 0x3F));
        buf[n++] = (char)(0x80 | ((cp >> 6) & 0x3F));
        buf[n++] = (char)(0x80 | (cp & 0x3F));
    }
    if (S.text_len + n + 1 > INPUT_TEXT_CAP) return;
    memcpy(S.text + S.text_len, buf, n);
    S.text_len += n;
    S.text[S.text_len] = '\0';
}

void input_push(const InputEvent* in) {
    if (!in) return;

    InputEvent ev = *in;
    if (ev.time == 0.0) ev.time = S.now;

    switch (ev.kind) {
        case INPUT_EV_MOUSE_MOVE:
            S.mouse = ev.pos;
            break;

        case INPUT_EV_MOUSE_DOWN:
            if (ev.button >= 0 && ev.button < INPUT_MOUSE_COUNT) {
                S.mouse = ev.pos;
                S.down[ev.button] = true;
                S.pressed[ev.button] = true;
                S.press_origin[ev.button] = ev.pos;

                if (ev.button == INPUT_MOUSE_LEFT) {
                    float dx = ev.pos.x - S.last_click_pos.x;
                    float dy = ev.pos.y - S.last_click_pos.y;
                    bool near = (fabsf(dx) <= S.click_slop && fabsf(dy) <= S.click_slop);
                    bool soon = (ev.time - S.last_click_time) <= S.click_window;
                    // Cycles 1 -> 2 -> 3 -> 1 so triple-click is reachable and
                    // a fourth click starts a fresh sequence.
                    S.click_count = (near && soon) ? (S.click_count % 3) + 1 : 1;
                    S.last_click_time = ev.time;
                    S.last_click_pos = ev.pos;
                }
                ev.click_count = S.click_count;
            }
            break;

        case INPUT_EV_MOUSE_UP:
            if (ev.button >= 0 && ev.button < INPUT_MOUSE_COUNT) {
                S.mouse = ev.pos;
                S.down[ev.button] = false;
                S.released[ev.button] = true;
            }
            break;

        case INPUT_EV_SCROLL:
            S.scroll.x += ev.scroll.x;
            S.scroll.y += ev.scroll.y;
            break;

        case INPUT_EV_KEY_DOWN:
        case INPUT_EV_KEY_REPEAT:
            if (ev.key > 0 && ev.key < INPUT_KEY_COUNT) {
                S.key_down[ev.key] = true;
                S.key_pressed[ev.key] = true;
            }
            S.mods = ev.mods;
            break;

        case INPUT_EV_KEY_UP:
            if (ev.key > 0 && ev.key < INPUT_KEY_COUNT) {
                S.key_down[ev.key] = false;
                S.key_released[ev.key] = true;
            }
            S.mods = ev.mods;
            break;

        case INPUT_EV_TEXT:
            append_utf8(ev.codepoint);
            break;
    }

    if (S.event_count < INPUT_MAX_EVENTS) S.events[S.event_count++] = ev;
}

// --- queries ---------------------------------------------------------------

double input_time_seconds(void) { return S.now; }

size_t input_event_count(void) { return S.event_count; }

const InputEvent* input_event_at(size_t i) {
    return i < S.event_count ? &S.events[i] : NULL;
}

Vec2 input_mouse(void)  { return S.mouse; }
Vec2 input_scroll(void) { return S.scroll; }
int  input_click_count(void) { return S.click_count; }

Vec2 input_mouse_delta(void) {
    return (Vec2){ S.mouse.x - S.mouse_prev.x, S.mouse.y - S.mouse_prev.y };
}

static bool valid_btn(int b) { return b >= 0 && b < INPUT_MOUSE_COUNT; }

bool input_mouse_down(int b)     { return valid_btn(b) && S.down[b]; }
bool input_mouse_pressed(int b)  { return valid_btn(b) && S.pressed[b]; }
bool input_mouse_released(int b) { return valid_btn(b) && S.released[b]; }
Vec2 input_press_origin(int b)   { return valid_btn(b) ? S.press_origin[b] : (Vec2){0,0}; }

static bool valid_key(int k) { return k > 0 && k < INPUT_KEY_COUNT; }

bool input_key_down(int k)     { return valid_key(k) && S.key_down[k]; }
bool input_key_pressed(int k)  { return valid_key(k) && S.key_pressed[k]; }
bool input_key_released(int k) { return valid_key(k) && S.key_released[k]; }
uint32_t input_mods(void)      { return S.mods; }
const char* input_text(void)   { return S.text; }

bool input_hover(Rect r) { return rect_contains(r, S.mouse.x, S.mouse.y); }

bool input_press_in(Rect r, int button) {
    return input_mouse_pressed(button) && rect_contains(r, S.mouse.x, S.mouse.y);
}

bool input_click_in(Rect r, int button) {
    if (!input_mouse_released(button)) return false;
    // Both ends of the gesture must be inside: a press that started elsewhere
    // and happened to end here is a drag, not a click.
    Vec2 origin = input_press_origin(button);
    return rect_contains(r, S.mouse.x, S.mouse.y) &&
           rect_contains(r, origin.x, origin.y);
}

// --- bindings --------------------------------------------------------------

static Binding* find_binding(const char* name) {
    if (!name) return NULL;
    for (size_t i = 0; i < INPUT_MAX_BINDINGS; i++) {
        if (S.bindings[i].name[0] && strcmp(S.bindings[i].name, name) == 0) {
            return &S.bindings[i];
        }
    }
    return NULL;
}

static Binding* alloc_binding(const char* name) {
    Binding* existing = find_binding(name);
    if (existing) return existing;
    for (size_t i = 0; i < INPUT_MAX_BINDINGS; i++) {
        if (!S.bindings[i].name[0]) return &S.bindings[i];
    }
    return NULL;
}

bool input_bind_key(const char* name, int key, uint32_t mods, InputAction fn, void* user) {
    if (!name || !fn || !valid_key(key)) return false;
    Binding* b = alloc_binding(name);
    if (!b) {
        fprintf(stderr, "[Input] binding table full, '%s' not registered\n", name);
        return false;
    }
    snprintf(b->name, sizeof(b->name), "%s", name);
    b->key = key;
    b->mods = mods;
    b->fn = fn;
    b->user = user;
    b->is_click = false;
    b->used = true;
    return true;
}

bool input_bind_click(const char* name, Rect area, InputAction fn, void* user) {
    if (!name || !fn) return false;
    Binding* b = alloc_binding(name);
    if (!b) return false;
    snprintf(b->name, sizeof(b->name), "%s", name);
    b->area = area;
    b->fn = fn;
    b->user = user;
    b->is_click = true;
    b->used = true;
    b->key = 0;
    b->mods = 0;
    return true;
}

void input_unbind(const char* name) {
    Binding* b = find_binding(name);
    if (b) memset(b, 0, sizeof(*b));
}

void input_clear_key_bindings(void) {
    for (size_t i = 0; i < INPUT_MAX_BINDINGS; i++) {
        if (!S.bindings[i].is_click) memset(&S.bindings[i], 0, sizeof(S.bindings[i]));
    }
}

void input_dispatch(void) {
    for (size_t i = 0; i < INPUT_MAX_BINDINGS; i++) {
        Binding* b = &S.bindings[i];
        if (!b->used || !b->fn) continue;

        if (b->is_click) {
            if (input_click_in(b->area, INPUT_MOUSE_LEFT)) b->fn(b->user);
            continue;
        }

        // Exact modifier match, so Ctrl+S does not also fire on Ctrl+Shift+S.
        if (input_key_pressed(b->key) && S.mods == b->mods) b->fn(b->user);
    }
}

size_t input_binding_count(void) {
    size_t n = 0;
    for (size_t i = 0; i < INPUT_MAX_BINDINGS; i++) {
        if (S.bindings[i].name[0]) n++;
    }
    return n;
}

bool input_binding_at(size_t index, InputBindingInfo* out) {
    if (!out) return false;
    size_t n = 0;
    for (size_t i = 0; i < INPUT_MAX_BINDINGS; i++) {
        if (!S.bindings[i].name[0]) continue;
        if (n++ != index) continue;
        out->name = S.bindings[i].name;
        out->key = S.bindings[i].key;
        out->mods = S.bindings[i].mods;
        out->is_click = S.bindings[i].is_click;
        return true;
    }
    return false;
}

static const char* key_name(int key) {
    static char one[2];
    switch (key) {
        case INPUT_KEY_ENTER:     return "Enter";
        case INPUT_KEY_ESCAPE:    return "Esc";
        case INPUT_KEY_TAB:       return "Tab";
        case INPUT_KEY_BACKSPACE: return "Backspace";
        case INPUT_KEY_DELETE:    return "Delete";
        case INPUT_KEY_SPACE:     return "Space";
        case INPUT_KEY_COMMA:     return ",";
        case INPUT_KEY_PERIOD:    return ".";
        case INPUT_KEY_SLASH:     return "/";
        case INPUT_KEY_LEFT:      return "Left";
        case INPUT_KEY_RIGHT:     return "Right";
        case INPUT_KEY_UP:        return "Up";
        case INPUT_KEY_DOWN:      return "Down";
        case INPUT_KEY_HOME:      return "Home";
        case INPUT_KEY_END:       return "End";
        case INPUT_KEY_PAGE_UP:   return "PageUp";
        case INPUT_KEY_PAGE_DOWN: return "PageDown";
        default:
            if (key >= INPUT_KEY_A && key <= INPUT_KEY_Z) {
                one[0] = (char)key;
                one[1] = '\0';
                return one;
            }
            if (key >= INPUT_KEY_0 && key <= INPUT_KEY_9) {
                one[0] = (char)key;
                one[1] = '\0';
                return one;
            }
            return "?";
    }
}

const char* input_binding_label(const InputBindingInfo* info, char* buf, size_t cap) {
    if (!buf || cap == 0) return "";
    buf[0] = '\0';
    if (!info) return buf;

    if (info->is_click) {
        snprintf(buf, cap, "Click");
        return buf;
    }

    size_t n = 0;
    if (info->mods & INPUT_MOD_CTRL)  n += (size_t)snprintf(buf + n, cap - n, "Ctrl+");
    if (n < cap && (info->mods & INPUT_MOD_ALT))   n += (size_t)snprintf(buf + n, cap - n, "Alt+");
    if (n < cap && (info->mods & INPUT_MOD_SHIFT)) n += (size_t)snprintf(buf + n, cap - n, "Shift+");
    if (n < cap && (info->mods & INPUT_MOD_SUPER)) n += (size_t)snprintf(buf + n, cap - n, "Super+");
    if (n < cap) snprintf(buf + n, cap - n, "%s", key_name(info->key));
    return buf;
}

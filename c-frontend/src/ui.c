#include "ui.h"
#include "gl_backend.h"
#include "json_utils.h"
#include "GLFW/glfw3.h"
#include <ctype.h>
#include <math.h>

UIContext g_ui = {0};

static float s_cursor_blink = 0.0f;
static char s_tooltip_text[512] = {0};
static bool s_has_tooltip = false;

// Context menu popup state
static uint64_t s_cm_active_id = 0;
static Rect s_cm_rect = {0, 0, 0, 0};

// Modal state
static bool s_modal_active = false;
static Rect s_modal_rect = {0, 0, 0, 0};

uint64_t ui_hash_id(const char* str, uint64_t seed) {
    if (!str) return seed;
    uint64_t hash = seed ^ 14695981039346656037ULL;
    while (*str) {
        hash ^= (uint8_t)*str++;
        hash *= 1099511628211ULL;
    }
    return hash;
}

void ui_init(int window_w, int window_h) {
    g_ui.window_width = window_w;
    g_ui.window_height = window_h;

    // Load crisp fonts
    g_ui.font_ui = font_load_default_ui(14.0f);
    g_ui.font_mono = font_load_default_mono(13.0f);
    g_ui.font_bold = font_load_default_ui(14.0f);

    theme_init();
}

void ui_shutdown(void) {
    if (g_ui.font_ui) font_free(g_ui.font_ui);
    if (g_ui.font_mono) font_free(g_ui.font_mono);
    if (g_ui.font_bold) font_free(g_ui.font_bold);
}

void ui_begin_frame(float dt) {
    g_ui.dt = dt;
    s_cursor_blink += dt * 3.0f;
    s_has_tooltip = false;
    s_tooltip_text[0] = '\0';
    g_ui.hover_id = 0;
}

void ui_end_frame(void) {
    Theme* t = theme_get_active();

    // Render tooltip if active
    if (s_has_tooltip && s_tooltip_text[0]) {
        float tw = font_measure_text(g_ui.font_ui, s_tooltip_text, 0) + 16.0f;
        float th = 24.0f;
        float tx = g_ui.mouse_x + 12.0f;
        float ty = g_ui.mouse_y + 16.0f;
        if (tx + tw > g_ui.window_width) tx = g_ui.window_width - tw - 4.0f;
        if (ty + th > g_ui.window_height) ty = g_ui.window_height - th - 4.0f;

        Rect tr = rect_make(tx, ty, tw, th);
        r_draw_rect_rounded_bordered(tr, 4.0f, 1.0f, t->border, t->panel_bg);
        font_draw_text(g_ui.font_ui, s_tooltip_text, 0, tx + 8.0f, ty + 5.0f, t->foreground);
    }

    // Reset single-frame inputs
    g_ui.mouse_clicked[0] = false;
    g_ui.mouse_clicked[1] = false;
    g_ui.mouse_clicked[2] = false;
    g_ui.mouse_released[0] = false;
    g_ui.mouse_released[1] = false;
    g_ui.mouse_released[2] = false;
    g_ui.scroll_x = 0;
    g_ui.scroll_y = 0;
    g_ui.key_pressed = 0;
    g_ui.char_input_len = 0;
}

void ui_set_tooltip(const char* text) {
    if (text) {
        strncpy(s_tooltip_text, text, sizeof(s_tooltip_text) - 1);
        s_has_tooltip = true;
    }
}

bool ui_button(const char* label, Rect rect, bool is_primary) {
    Theme* t = theme_get_active();
    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);
    bool clicked = hovered && g_ui.mouse_clicked[0];

    Color bg = is_primary ? (hovered ? t->primary_hover : t->primary) : (hovered ? t->secondary_hover : t->secondary);
    Color txt = is_primary ? t->primary_text : (hovered ? t->foreground : t->text_muted);
    Color bdr = is_primary ? t->primary : t->border;

    r_draw_rect_rounded_bordered(rect, 4.0f, 1.0f, bdr, bg);

    if (label) {
        float tw = font_measure_text(g_ui.font_bold, label, 0);
        float tx = rect.x + (rect.w - tw) * 0.5f;
        float ty = rect.y + (rect.h - 14.0f) * 0.5f;
        font_draw_text(g_ui.font_bold, label, 0, tx, ty, txt);
    }

    return clicked;
}

bool ui_button_ghost(const char* label, Rect rect) {
    Theme* t = theme_get_active();
    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);
    bool clicked = hovered && g_ui.mouse_clicked[0];

    if (hovered) {
        r_draw_rect_rounded(rect, 4.0f, t->secondary_hover);
    }

    if (label) {
        float tw = font_measure_text(g_ui.font_ui, label, 0);
        float tx = rect.x + (rect.w - tw) * 0.5f;
        float ty = rect.y + (rect.h - 14.0f) * 0.5f;
        font_draw_text(g_ui.font_ui, label, 0, tx, ty, hovered ? t->foreground : t->text_muted);
    }

    return clicked;
}

bool ui_button_danger(const char* label, Rect rect) {
    Theme* t = theme_get_active();
    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);
    bool clicked = hovered && g_ui.mouse_clicked[0];

    Color bg = hovered ? t->danger_hover : t->danger;
    r_draw_rect_rounded(rect, 4.0f, bg);

    if (label) {
        float tw = font_measure_text(g_ui.font_bold, label, 0);
        float tx = rect.x + (rect.w - tw) * 0.5f;
        float ty = rect.y + (rect.h - 14.0f) * 0.5f;
        font_draw_text(g_ui.font_bold, label, 0, tx, ty, color_hex(0xFFFFFF));
    }

    return clicked;
}

bool ui_button_icon(IconType icon, const char* label, Rect rect, bool is_active) {
    Theme* t = theme_get_active();
    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);
    bool clicked = hovered && g_ui.mouse_clicked[0];

    Color bg = is_active ? t->secondary_active : (hovered ? t->secondary_hover : color_rgba(0,0,0,0));
    Color col = is_active ? t->primary : (hovered ? t->foreground : t->text_muted);

    if (bg.a > 0.01f) {
        r_draw_rect_rounded(rect, 4.0f, bg);
    }

    float cur_x = rect.x + (label ? 6.0f : (rect.w - 14.0f) * 0.5f);
    if (icon != ICON_NONE) {
        Rect icon_rect = rect_make(cur_x, rect.y + (rect.h - 14.0f) * 0.5f, 14.0f, 14.0f);
        r_draw_icon(icon, icon_rect, col);
        cur_x += 18.0f;
    }

    if (label) {
        float ty = rect.y + (rect.h - 14.0f) * 0.5f;
        font_draw_text(g_ui.font_ui, label, 0, cur_x, ty, col);
    }

    return clicked;
}

bool ui_checkbox(const char* label, bool* checked, Rect rect) {
    Theme* t = theme_get_active();
    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);
    bool clicked = hovered && g_ui.mouse_clicked[0];

    if (clicked && checked) {
        *checked = !*checked;
    }

    Rect box = rect_make(rect.x + 2.0f, rect.y + (rect.h - 14.0f) * 0.5f, 14.0f, 14.0f);
    Color bg = (checked && *checked) ? t->primary : t->input_bg;
    Color bdr = (hovered || (checked && *checked)) ? t->primary : t->border;

    r_draw_rect_rounded_bordered(box, 3.0f, 1.0f, bdr, bg);

    if (checked && *checked) {
        r_draw_icon(ICON_CHECK, box, t->primary_text);
    }

    if (label) {
        float tx = box.x + box.w + 6.0f;
        float ty = rect.y + (rect.h - 14.0f) * 0.5f;
        font_draw_text(g_ui.font_ui, label, 0, tx, ty, hovered ? t->foreground : t->text_muted);
    }

    return clicked;
}

static size_t find_word_prev(const char* str, size_t pos) {
    if (pos == 0) return 0;
    pos--;
    while (pos > 0 && isspace((unsigned char)str[pos])) pos--;
    while (pos > 0 && !isspace((unsigned char)str[pos - 1])) pos--;
    return pos;
}

static size_t find_word_next(const char* str, size_t len, size_t pos) {
    while (pos < len && !isspace((unsigned char)str[pos])) pos++;
    while (pos < len && isspace((unsigned char)str[pos])) pos++;
    return pos;
}

static uint64_t s_input_active_id = 0;
static size_t s_input_cursor = 0;
static size_t s_input_anchor = 0;
static bool s_input_is_dragging = false;

bool ui_input_text(const char* id_str, char* buffer, size_t buffer_size, const char* placeholder, Rect rect) {
    Theme* t = theme_get_active();
    uint64_t id = ui_hash_id(id_str, 0);
    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);
    bool is_focused = (g_ui.focus_id == id);

    size_t len = buffer ? strlen(buffer) : 0;
    float tx = rect.x + 8.0f;
    float ty = rect.y + (rect.h - 14.0f) * 0.5f;

    if (hovered && g_ui.mouse_clicked[0]) {
        g_ui.focus_id = id;
        is_focused = true;
        s_input_active_id = id;

        // Calculate cursor pos from click
        float cx = tx;
        size_t clicked_pos = len;
        for (size_t i = 0; i < len; i++) {
            float cw = font_measure_char(g_ui.font_ui, buffer[i]);
            if (g_ui.mouse_x < cx + cw * 0.5f) {
                clicked_pos = i;
                break;
            }
            cx += cw;
        }
        s_input_cursor = clicked_pos;
        s_input_anchor = clicked_pos;
        s_input_is_dragging = true;
    } else if (!hovered && g_ui.mouse_clicked[0] && is_focused) {
        g_ui.focus_id = 0;
        is_focused = false;
        s_input_is_dragging = false;
    }

    if (s_input_is_dragging && is_focused && s_input_active_id == id && g_ui.mouse_down[0]) {
        float cx = tx;
        size_t drag_pos = len;
        for (size_t i = 0; i < len; i++) {
            float cw = font_measure_char(g_ui.font_ui, buffer[i]);
            if (g_ui.mouse_x < cx + cw * 0.5f) {
                drag_pos = i;
                break;
            }
            cx += cw;
        }
        s_input_cursor = drag_pos;
    }

    if (g_ui.mouse_released[0]) {
        s_input_is_dragging = false;
    }

    bool changed = false;
    if (is_focused && buffer && buffer_size > 0) {
        if (s_input_cursor > len) s_input_cursor = len;
        if (s_input_anchor > len) s_input_anchor = len;

        size_t s_min = s_input_cursor < s_input_anchor ? s_input_cursor : s_input_anchor;
        size_t s_max = s_input_cursor > s_input_anchor ? s_input_cursor : s_input_anchor;
        bool has_sel = (s_min != s_max);

        // Arrow navigation
        if (g_ui.key_pressed == GLFW_KEY_LEFT) {
            if (g_ui.ctrl_down) {
                s_input_cursor = find_word_prev(buffer, s_input_cursor);
            } else if (s_input_cursor > 0) {
                s_input_cursor--;
            }
            if (!g_ui.shift_down) s_input_anchor = s_input_cursor;
        } else if (g_ui.key_pressed == GLFW_KEY_RIGHT) {
            if (g_ui.ctrl_down) {
                s_input_cursor = find_word_next(buffer, len, s_input_cursor);
            } else if (s_input_cursor < len) {
                s_input_cursor++;
            }
            if (!g_ui.shift_down) s_input_anchor = s_input_cursor;
        } else if (g_ui.key_pressed == GLFW_KEY_HOME) {
            s_input_cursor = 0;
            if (!g_ui.shift_down) s_input_anchor = 0;
        } else if (g_ui.key_pressed == GLFW_KEY_END) {
            s_input_cursor = len;
            if (!g_ui.shift_down) s_input_anchor = len;
        }

        // Selection & Clipboard shortcuts
        if (g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_A) {
            s_input_anchor = 0;
            s_input_cursor = len;
        } else if (g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_C) {
            if (has_sel) {
                size_t slen = s_max - s_min;
                char* clip = malloc(slen + 1);
                memcpy(clip, buffer + s_min, slen);
                clip[slen] = '\0';
                glfwSetClipboardString(NULL, clip);
                free(clip);
            } else {
                glfwSetClipboardString(NULL, buffer);
            }
        } else if (g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_X) {
            if (has_sel) {
                size_t slen = s_max - s_min;
                char* clip = malloc(slen + 1);
                memcpy(clip, buffer + s_min, slen);
                clip[slen] = '\0';
                glfwSetClipboardString(NULL, clip);
                free(clip);
                memmove(buffer + s_min, buffer + s_max, len - s_max + 1);
                s_input_cursor = s_input_anchor = s_min;
                changed = true;
                len = strlen(buffer);
            }
        } else if (g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_V) {
            const char* clip = glfwGetClipboardString(NULL);
            if (clip && *clip) {
                if (has_sel) {
                    memmove(buffer + s_min, buffer + s_max, len - s_max + 1);
                    s_input_cursor = s_input_anchor = s_min;
                    len = strlen(buffer);
                }
                size_t clen = strlen(clip);
                if (len + clen < buffer_size) {
                    memmove(buffer + s_input_cursor + clen, buffer + s_input_cursor, len - s_input_cursor + 1);
                    memcpy(buffer + s_input_cursor, clip, clen);
                    s_input_cursor = s_input_anchor = s_input_cursor + clen;
                    changed = true;
                    len = strlen(buffer);
                }
            }
        } else if (g_ui.key_pressed == GLFW_KEY_BACKSPACE) {
            if (has_sel) {
                memmove(buffer + s_min, buffer + s_max, len - s_max + 1);
                s_input_cursor = s_input_anchor = s_min;
                changed = true;
                len = strlen(buffer);
            } else if (s_input_cursor > 0) {
                memmove(buffer + s_input_cursor - 1, buffer + s_input_cursor, len - s_input_cursor + 1);
                s_input_cursor--;
                s_input_anchor = s_input_cursor;
                changed = true;
                len = strlen(buffer);
            }
        } else if (g_ui.key_pressed == GLFW_KEY_DELETE) {
            if (has_sel) {
                memmove(buffer + s_min, buffer + s_max, len - s_max + 1);
                s_input_cursor = s_input_anchor = s_min;
                changed = true;
                len = strlen(buffer);
            } else if (s_input_cursor < len) {
                memmove(buffer + s_input_cursor, buffer + s_input_cursor + 1, len - s_input_cursor);
                changed = true;
                len = strlen(buffer);
            }
        }

        // Character Typing Input
        if (g_ui.char_input_len > 0) {
            if (has_sel) {
                memmove(buffer + s_min, buffer + s_max, len - s_max + 1);
                s_input_cursor = s_input_anchor = s_min;
                len = strlen(buffer);
            }
            size_t in_len = 0;
            char temp_in[32];
            for (size_t i = 0; i < g_ui.char_input_len; i++) {
                if (g_ui.char_input[i] >= 32) {
                    temp_in[in_len++] = g_ui.char_input[i];
                }
            }
            if (in_len > 0 && len + in_len < buffer_size) {
                memmove(buffer + s_input_cursor + in_len, buffer + s_input_cursor, len - s_input_cursor + 1);
                memcpy(buffer + s_input_cursor, temp_in, in_len);
                s_input_cursor = s_input_anchor = s_input_cursor + in_len;
                changed = true;
                len = strlen(buffer);
            }
        }
    }

    Color bg = t->input_bg;
    Color bdr = is_focused ? t->input_border_focus : (hovered ? t->border : t->input_border);

    r_draw_rect_rounded_bordered(rect, 4.0f, 1.0f, bdr, bg);
    r_push_clip(rect_make(rect.x + 4.0f, rect.y + 1.0f, rect.w - 8.0f, rect.h - 2.0f));

    if (buffer && buffer[0]) {
        size_t s_min = (is_focused && s_input_active_id == id) ? (s_input_cursor < s_input_anchor ? s_input_cursor : s_input_anchor) : 0;
        size_t s_max = (is_focused && s_input_active_id == id) ? (s_input_cursor > s_input_anchor ? s_input_cursor : s_input_anchor) : 0;
        bool has_sel = (s_min != s_max);

        // Draw selection background
        if (has_sel) {
            float sx0 = tx + font_measure_text(g_ui.font_ui, buffer, s_min);
            float sx1 = tx + font_measure_text(g_ui.font_ui, buffer, s_max);
            r_draw_rect_rounded(rect_make(sx0, ty - 1.0f, sx1 - sx0, 16.0f), 2.0f, t->selection_bg);
        }

        const char* p = buffer;
        float cx = tx;
        while (*p) {
            if (p[0] == '{' && p[1] == '{') {
                const char* end = strstr(p, "}}");
                if (end) {
                    size_t var_len = (end + 2) - p;
                    float vw = font_measure_text(g_ui.font_ui, p, var_len);
                    r_draw_rect_rounded(rect_make(cx - 2.0f, ty - 1.0f, vw + 4.0f, 16.0f), 3.0f, color_with_alpha(t->warning, 0.2f));
                    font_draw_text(g_ui.font_ui, p, var_len, cx, ty, t->warning);
                    cx += vw;
                    p = end + 2;
                    continue;
                }
            }
            // Non-variable chunk
            const char* next_var = strstr(p, "{{");
            size_t chunk_len = next_var ? (size_t)(next_var - p) : strlen(p);
            float cw = font_measure_text(g_ui.font_ui, p, chunk_len);
            font_draw_text(g_ui.font_ui, p, chunk_len, cx, ty, t->foreground);
            cx += cw;
            p += chunk_len;
        }

        // Blinking Cursor
        if (is_focused && ((int)s_cursor_blink % 2 == 0)) {
            float cursor_x = tx + font_measure_text(g_ui.font_ui, buffer, s_input_cursor);
            r_draw_line((Vec2){cursor_x, ty}, (Vec2){cursor_x, ty + 14.0f}, 1.5f, t->primary);
        }
    } else {
        if (placeholder) {
            font_draw_text(g_ui.font_ui, placeholder, 0, tx, ty, t->input_placeholder);
        }
        if (is_focused && ((int)s_cursor_blink % 2 == 0)) {
            r_draw_line((Vec2){tx, ty}, (Vec2){tx, ty + 14.0f}, 1.5f, t->primary);
        }
    }

    r_pop_clip();
    return changed;
}

static size_t ui_calc_editor_offset(const char* text, size_t len, float start_y, float text_x, float line_h, float mouse_x, float mouse_y, Font* font) {
    if (!text || len == 0) return 0;
    int target_line = (int)((mouse_y - start_y) / line_h);
    if (target_line < 0) return 0;

    int cur_line = 0;
    size_t line_start = 0;
    for (size_t i = 0; i <= len; i++) {
        if (i == len || text[i] == '\n') {
            if (cur_line == target_line || i == len) {
                float cx = text_x;
                for (size_t k = line_start; k < i; k++) {
                    float cw = font_measure_char(font, text[k]);
                    if (mouse_x < cx + cw * 0.5f) {
                        return k;
                    }
                    cx += cw;
                }
                return i;
            }
            line_start = i + 1;
            cur_line++;
        }
    }
    return len;
}

static void editor_get_line_col(const char* text, size_t len, size_t offset, size_t* out_line, size_t* out_col, size_t* out_line_start) {
    size_t line = 0;
    size_t line_start = 0;
    for (size_t i = 0; i <= len; i++) {
        if (i == offset) {
            if (out_line) *out_line = line;
            if (out_col) *out_col = i - line_start;
            if (out_line_start) *out_line_start = line_start;
            return;
        }
        if (i == len || text[i] == '\n') {
            line++;
            line_start = i + 1;
        }
    }
    if (out_line) *out_line = line;
    if (out_col) *out_col = len - line_start;
    if (out_line_start) *out_line_start = line_start;
}

static size_t editor_line_col_to_offset(const char* text, size_t len, size_t target_line, size_t target_col) {
    size_t line = 0;
    size_t line_start = 0;
    for (size_t i = 0; i <= len; i++) {
        if (i == len || text[i] == '\n') {
            if (line == target_line) {
                size_t line_len = i - line_start;
                size_t c = (target_col < line_len) ? target_col : line_len;
                return line_start + c;
            }
            line++;
            line_start = i + 1;
        }
    }
    return len;
}

static void editor_ensure_cap(char** content_ptr, size_t* content_cap, size_t needed) {
    if (needed + 1 >= *content_cap) {
        size_t new_cap = (needed + 512) * 2;
        *content_ptr = realloc(*content_ptr, new_cap);
        *content_cap = new_cap;
    }
}

bool ui_code_editor(const char* id_str, char** content_ptr, size_t* content_len, size_t* content_cap,
                    bool show_line_numbers, bool is_json_syntax, bool is_readonly, Rect rect, float* scroll_y) {
    Theme* t = theme_get_active();
    uint64_t id = ui_hash_id(id_str, 0);
    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);
    bool is_focused = (g_ui.focus_id == id);

    if (!content_ptr || !*content_ptr) {
        static char* empty_str = "";
        if (!content_ptr) content_ptr = &empty_str;
        else if (!*content_ptr) {
            *content_ptr = calloc(1, 1024);
            if (content_cap) *content_cap = 1024;
            if (content_len) *content_len = 0;
        }
    }

    const char* text = *content_ptr;
    size_t len = strlen(text);
    if (content_len) *content_len = len;

    float gutter_w = show_line_numbers ? 42.0f : 8.0f;
    float line_h = 18.0f;
    float start_y = rect.y + 6.0f - (scroll_y ? *scroll_y : 0.0f);
    float text_x = rect.x + gutter_w + 10.0f;

    if (hovered && scroll_y) {
        *scroll_y -= g_ui.scroll_y * 24.0f;
        if (*scroll_y < 0.0f) *scroll_y = 0.0f;
    }

    // Mouse selection logic
    if (hovered && g_ui.mouse_clicked[0]) {
        g_ui.focus_id = id;
        is_focused = true;
        g_ui.editor_sel_id = id;
        size_t off = ui_calc_editor_offset(text, len, start_y, text_x, line_h, g_ui.mouse_x, g_ui.mouse_y, g_ui.font_mono);
        g_ui.sel_start = off;
        g_ui.sel_end = off;
        g_ui.is_selecting = true;
    }

    if (g_ui.is_selecting && g_ui.editor_sel_id == id && g_ui.mouse_down[0]) {
        size_t off = ui_calc_editor_offset(text, len, start_y, text_x, line_h, g_ui.mouse_x, g_ui.mouse_y, g_ui.font_mono);
        g_ui.sel_end = off;
    }

    if (g_ui.mouse_released[0]) {
        g_ui.is_selecting = false;
    }

    bool changed = false;

    // Keyboard Shortcuts and Editing
    if (is_focused) {
        if (g_ui.sel_end > len) g_ui.sel_end = len;
        if (g_ui.sel_start > len) g_ui.sel_start = len;

        size_t s_min = g_ui.sel_start < g_ui.sel_end ? g_ui.sel_start : g_ui.sel_end;
        size_t s_max = g_ui.sel_start > g_ui.sel_end ? g_ui.sel_start : g_ui.sel_end;
        bool has_sel = (s_min != s_max);

        // Arrow navigation
        if (g_ui.key_pressed == GLFW_KEY_LEFT) {
            if (g_ui.ctrl_down) {
                g_ui.sel_end = find_word_prev(text, g_ui.sel_end);
            } else if (g_ui.sel_end > 0) {
                g_ui.sel_end--;
            }
            if (!g_ui.shift_down) g_ui.sel_start = g_ui.sel_end;
        } else if (g_ui.key_pressed == GLFW_KEY_RIGHT) {
            if (g_ui.ctrl_down) {
                g_ui.sel_end = find_word_next(text, len, g_ui.sel_end);
            } else if (g_ui.sel_end < len) {
                g_ui.sel_end++;
            }
            if (!g_ui.shift_down) g_ui.sel_start = g_ui.sel_end;
        } else if (g_ui.key_pressed == GLFW_KEY_UP) {
            size_t cur_line = 0, cur_col = 0, lstart = 0;
            editor_get_line_col(text, len, g_ui.sel_end, &cur_line, &cur_col, &lstart);
            if (cur_line > 0) {
                g_ui.sel_end = editor_line_col_to_offset(text, len, cur_line - 1, cur_col);
            } else {
                g_ui.sel_end = 0;
            }
            if (!g_ui.shift_down) g_ui.sel_start = g_ui.sel_end;
        } else if (g_ui.key_pressed == GLFW_KEY_DOWN) {
            size_t cur_line = 0, cur_col = 0, lstart = 0;
            editor_get_line_col(text, len, g_ui.sel_end, &cur_line, &cur_col, &lstart);
            g_ui.sel_end = editor_line_col_to_offset(text, len, cur_line + 1, cur_col);
            if (!g_ui.shift_down) g_ui.sel_start = g_ui.sel_end;
        } else if (g_ui.key_pressed == GLFW_KEY_HOME) {
            size_t cur_line = 0, cur_col = 0, lstart = 0;
            editor_get_line_col(text, len, g_ui.sel_end, &cur_line, &cur_col, &lstart);
            g_ui.sel_end = g_ui.ctrl_down ? 0 : lstart;
            if (!g_ui.shift_down) g_ui.sel_start = g_ui.sel_end;
        } else if (g_ui.key_pressed == GLFW_KEY_END) {
            size_t cur_line = 0, cur_col = 0, lstart = 0;
            editor_get_line_col(text, len, g_ui.sel_end, &cur_line, &cur_col, &lstart);
            size_t lend = lstart;
            while (lend < len && text[lend] != '\n') lend++;
            g_ui.sel_end = g_ui.ctrl_down ? len : lend;
            if (!g_ui.shift_down) g_ui.sel_start = g_ui.sel_end;
        }

        // Copy (Ctrl+C)
        if (g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_C) {
            if (has_sel && s_max <= len) {
                size_t slen = s_max - s_min;
                char* clip_buf = malloc(slen + 1);
                memcpy(clip_buf, text + s_min, slen);
                clip_buf[slen] = '\0';
                glfwSetClipboardString(NULL, clip_buf);
                free(clip_buf);
            } else {
                glfwSetClipboardString(NULL, text);
            }
        } else if (g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_A) {
            g_ui.editor_sel_id = id;
            g_ui.sel_start = 0;
            g_ui.sel_end = len;
        }

        // Editable operations when !is_readonly
        if (!is_readonly && content_cap) {
            // Cut (Ctrl+X)
            if (g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_X && has_sel) {
                size_t slen = s_max - s_min;
                char* clip_buf = malloc(slen + 1);
                memcpy(clip_buf, text + s_min, slen);
                clip_buf[slen] = '\0';
                glfwSetClipboardString(NULL, clip_buf);
                free(clip_buf);

                memmove((*content_ptr) + s_min, (*content_ptr) + s_max, len - s_max + 1);
                g_ui.sel_start = g_ui.sel_end = s_min;
                changed = true;
                len = strlen(*content_ptr);
                if (content_len) *content_len = len;
            }
            // Paste (Ctrl+V)
            else if (g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_V) {
                const char* clip = glfwGetClipboardString(NULL);
                if (clip && *clip) {
                    if (has_sel) {
                        memmove((*content_ptr) + s_min, (*content_ptr) + s_max, len - s_max + 1);
                        g_ui.sel_start = g_ui.sel_end = s_min;
                        len = strlen(*content_ptr);
                    }
                    size_t clen = strlen(clip);
                    editor_ensure_cap(content_ptr, content_cap, len + clen);
                    memmove((*content_ptr) + g_ui.sel_end + clen, (*content_ptr) + g_ui.sel_end, len - g_ui.sel_end + 1);
                    memcpy((*content_ptr) + g_ui.sel_end, clip, clen);
                    g_ui.sel_start = g_ui.sel_end = g_ui.sel_end + clen;
                    changed = true;
                    len = strlen(*content_ptr);
                    if (content_len) *content_len = len;
                }
            }
            // Backspace
            else if (g_ui.key_pressed == GLFW_KEY_BACKSPACE) {
                if (has_sel) {
                    memmove((*content_ptr) + s_min, (*content_ptr) + s_max, len - s_max + 1);
                    g_ui.sel_start = g_ui.sel_end = s_min;
                    changed = true;
                    len = strlen(*content_ptr);
                    if (content_len) *content_len = len;
                } else if (g_ui.sel_end > 0) {
                    memmove((*content_ptr) + g_ui.sel_end - 1, (*content_ptr) + g_ui.sel_end, len - g_ui.sel_end + 1);
                    g_ui.sel_end--;
                    g_ui.sel_start = g_ui.sel_end;
                    changed = true;
                    len = strlen(*content_ptr);
                    if (content_len) *content_len = len;
                }
            }
            // Delete
            else if (g_ui.key_pressed == GLFW_KEY_DELETE) {
                if (has_sel) {
                    memmove((*content_ptr) + s_min, (*content_ptr) + s_max, len - s_max + 1);
                    g_ui.sel_start = g_ui.sel_end = s_min;
                    changed = true;
                    len = strlen(*content_ptr);
                    if (content_len) *content_len = len;
                } else if (g_ui.sel_end < len) {
                    memmove((*content_ptr) + g_ui.sel_end, (*content_ptr) + g_ui.sel_end + 1, len - g_ui.sel_end);
                    changed = true;
                    len = strlen(*content_ptr);
                    if (content_len) *content_len = len;
                }
            }
            // Enter key (newline + auto indent)
            else if (g_ui.key_pressed == GLFW_KEY_ENTER) {
                if (has_sel) {
                    memmove((*content_ptr) + s_min, (*content_ptr) + s_max, len - s_max + 1);
                    g_ui.sel_start = g_ui.sel_end = s_min;
                    len = strlen(*content_ptr);
                }
                size_t cur_line = 0, cur_col = 0, lstart = 0;
                editor_get_line_col(*content_ptr, len, g_ui.sel_end, &cur_line, &cur_col, &lstart);
                size_t indent_len = 0;
                while (lstart + indent_len < len && ((*content_ptr)[lstart + indent_len] == ' ' || (*content_ptr)[lstart + indent_len] == '\t')) {
                    indent_len++;
                }
                editor_ensure_cap(content_ptr, content_cap, len + 1 + indent_len);
                memmove((*content_ptr) + g_ui.sel_end + 1 + indent_len, (*content_ptr) + g_ui.sel_end, len - g_ui.sel_end + 1);
                (*content_ptr)[g_ui.sel_end] = '\n';
                if (indent_len > 0) {
                    memcpy((*content_ptr) + g_ui.sel_end + 1, (*content_ptr) + lstart, indent_len);
                }
                g_ui.sel_start = g_ui.sel_end = g_ui.sel_end + 1 + indent_len;
                changed = true;
                len = strlen(*content_ptr);
                if (content_len) *content_len = len;
            }
            // Tab key (2 spaces)
            else if (g_ui.key_pressed == GLFW_KEY_TAB) {
                if (has_sel) {
                    memmove((*content_ptr) + s_min, (*content_ptr) + s_max, len - s_max + 1);
                    g_ui.sel_start = g_ui.sel_end = s_min;
                    len = strlen(*content_ptr);
                }
                editor_ensure_cap(content_ptr, content_cap, len + 2);
                memmove((*content_ptr) + g_ui.sel_end + 2, (*content_ptr) + g_ui.sel_end, len - g_ui.sel_end + 1);
                (*content_ptr)[g_ui.sel_end] = ' ';
                (*content_ptr)[g_ui.sel_end + 1] = ' ';
                g_ui.sel_start = g_ui.sel_end = g_ui.sel_end + 2;
                changed = true;
                len = strlen(*content_ptr);
                if (content_len) *content_len = len;
            }
            // Character typing
            else if (g_ui.char_input_len > 0) {
                if (has_sel) {
                    memmove((*content_ptr) + s_min, (*content_ptr) + s_max, len - s_max + 1);
                    g_ui.sel_start = g_ui.sel_end = s_min;
                    len = strlen(*content_ptr);
                }
                size_t in_len = 0;
                char temp_in[32];
                for (size_t i = 0; i < g_ui.char_input_len; i++) {
                    if (g_ui.char_input[i] >= 32) {
                        temp_in[in_len++] = g_ui.char_input[i];
                    }
                }
                if (in_len > 0) {
                    editor_ensure_cap(content_ptr, content_cap, len + in_len);
                    memmove((*content_ptr) + g_ui.sel_end + in_len, (*content_ptr) + g_ui.sel_end, len - g_ui.sel_end + 1);
                    memcpy((*content_ptr) + g_ui.sel_end, temp_in, in_len);
                    g_ui.sel_start = g_ui.sel_end = g_ui.sel_end + in_len;
                    changed = true;
                    len = strlen(*content_ptr);
                    if (content_len) *content_len = len;
                }
            }
        }
    }

    text = *content_ptr;
    len = strlen(text);

    r_draw_rect(rect, t->background);

    if (show_line_numbers) {
        r_draw_rect(rect_make(rect.x, rect.y, gutter_w, rect.h), t->panel_bg);
        r_draw_line((Vec2){rect.x + gutter_w, rect.y}, (Vec2){rect.x + gutter_w, rect.y + rect.h}, 1.0f, t->border);
    }

    r_push_clip(rect_make(rect.x, rect.y, rect.w, rect.h));

    size_t token_count = 0;
    JsonToken* tokens = is_json_syntax ? json_tokenize(text, len, &token_count) : NULL;

    int line_num = 1;
    size_t line_start = 0;
    size_t s_min = (g_ui.editor_sel_id == id && g_ui.sel_start != g_ui.sel_end) ? (g_ui.sel_start < g_ui.sel_end ? g_ui.sel_start : g_ui.sel_end) : 0;
    size_t s_max = (g_ui.editor_sel_id == id && g_ui.sel_start != g_ui.sel_end) ? (g_ui.sel_start > g_ui.sel_end ? g_ui.sel_start : g_ui.sel_end) : 0;
    bool has_sel = (s_min != s_max);

    float cursor_draw_x = -1.0f;
    float cursor_draw_y = -1.0f;

    for (size_t i = 0; i <= len; i++) {
        if (i == len || text[i] == '\n') {
            float cur_y = start_y + (line_num - 1) * line_h;

            if (cur_y + line_h >= rect.y && cur_y <= rect.y + rect.h) {
                if (show_line_numbers) {
                    char lbuf[16];
                    snprintf(lbuf, sizeof(lbuf), "%d", line_num);
                    float lw = font_measure_text(g_ui.font_mono, lbuf, 0);
                    font_draw_text(g_ui.font_mono, lbuf, 0, rect.x + gutter_w - lw - 8.0f, cur_y, t->text_dim);
                }

                if (has_sel && line_start < s_max && i >= s_min) {
                    size_t sel_l0 = (line_start > s_min) ? line_start : s_min;
                    size_t sel_l1 = (i < s_max) ? i : s_max;
                    float x0 = text_x + font_measure_text(g_ui.font_mono, text + line_start, sel_l0 - line_start);
                    float x1 = text_x + font_measure_text(g_ui.font_mono, text + line_start, sel_l1 - line_start);
                    if (x1 <= x0) x1 = x0 + 6.0f;
                    r_draw_rect_rounded(rect_make(x0, cur_y, x1 - x0, line_h), 2.0f, t->selection_bg);
                }

                if (is_focused && g_ui.editor_sel_id == id && g_ui.sel_end >= line_start && g_ui.sel_end <= i) {
                    cursor_draw_x = text_x + font_measure_text(g_ui.font_mono, text + line_start, g_ui.sel_end - line_start);
                    cursor_draw_y = cur_y;
                }

                size_t line_len = i - line_start;
                if (tokens && token_count > 0) {
                    float cur_x = text_x;
                    size_t k = line_start;
                    while (k < i) {
                        JsonToken* tok_match = NULL;
                        for (size_t ti = 0; ti < token_count; ti++) {
                            if (tokens[ti].start <= k && tokens[ti].start + tokens[ti].length > k) {
                                tok_match = &tokens[ti];
                                break;
                            }
                        }

                        if (tok_match) {
                            size_t tok_rem = (tok_match->start + tok_match->length) - k;
                            if (k + tok_rem > i) tok_rem = i - k;

                            Color tok_col = t->foreground;
                            switch (tok_match->type) {
                                case JSON_TOK_KEY: tok_col = t->syntax_key; break;
                                case JSON_TOK_STRING: tok_col = t->syntax_string; break;
                                case JSON_TOK_NUMBER: tok_col = t->syntax_number; break;
                                case JSON_TOK_BOOLEAN: tok_col = t->syntax_boolean; break;
                                case JSON_TOK_NULL: tok_col = t->syntax_null; break;
                                case JSON_TOK_PUNCTUATION: tok_col = t->syntax_punctuation; break;
                                case JSON_TOK_COMMENT: tok_col = t->syntax_comment; break;
                                default: break;
                            }

                            font_draw_text(g_ui.font_mono, text + k, tok_rem, cur_x, cur_y, tok_col);
                            cur_x += font_measure_text(g_ui.font_mono, text + k, tok_rem);
                            k += tok_rem;
                        } else {
                            size_t next_k = i;
                            for (size_t ti = 0; ti < token_count; ti++) {
                                if (tokens[ti].start > k && tokens[ti].start < next_k) {
                                    next_k = tokens[ti].start;
                                }
                            }
                            size_t plain_len = next_k - k;
                            font_draw_text(g_ui.font_mono, text + k, plain_len, cur_x, cur_y, t->foreground);
                            cur_x += font_measure_text(g_ui.font_mono, text + k, plain_len);
                            k += plain_len;
                        }
                    }
                } else {
                    font_draw_text(g_ui.font_mono, text + line_start, line_len, text_x, cur_y, t->foreground);
                }
            }

            line_start = i + 1;
            line_num++;
        }
    }

    if (tokens) free(tokens);

    // Blinking Caret
    if (is_focused && cursor_draw_x >= 0.0f && ((int)s_cursor_blink % 2 == 0)) {
        r_draw_line((Vec2){cursor_draw_x, cursor_draw_y}, (Vec2){cursor_draw_x, cursor_draw_y + line_h}, 2.0f, t->primary);
    }

    // Top-Right Floating Copy Button
    Rect copy_btn = rect_make(rect.x + rect.w - 65.0f, rect.y + 6.0f, 55.0f, 22.0f);
    if (ui_button_icon(ICON_NONE, "Copy", copy_btn, false)) {
        glfwSetClipboardString(NULL, text);
    }

    r_pop_clip();
    return changed;
}

int ui_tab_bar(const char* id_str, const char** tab_labels, int tab_count, int active_index, Rect rect) {
    (void)id_str;
    Theme* t = theme_get_active();
    r_draw_rect(rect, t->tabbar_bg);
    r_draw_line((Vec2){rect.x, rect.y + rect.h}, (Vec2){rect.x + rect.w, rect.y + rect.h}, 1.0f, t->border);

    float cur_x = rect.x + 8.0f;
    int clicked_index = active_index;

    for (int i = 0; i < tab_count; i++) {
        const char* label = tab_labels[i];
        float tw = font_measure_text(g_ui.font_ui, label, 0);
        float tab_w = tw + 20.0f;
        Rect tr = rect_make(cur_x, rect.y, tab_w, rect.h);

        bool is_active = (i == active_index);
        bool hovered = rect_contains(tr, g_ui.mouse_x, g_ui.mouse_y);

        if (hovered && g_ui.mouse_clicked[0]) {
            clicked_index = i;
        }

        if (is_active) {
            r_draw_rect(tr, t->tab_active_bg);
            r_draw_line((Vec2){tr.x, tr.y + tr.h - 2.0f}, (Vec2){tr.x + tr.w, tr.y + tr.h - 2.0f}, 2.0f, t->primary);
        } else if (hovered) {
            r_draw_rect(tr, t->secondary_hover);
        }

        float tx = tr.x + 10.0f;
        float ty = tr.y + (tr.h - 14.0f) * 0.5f;
        font_draw_text(g_ui.font_ui, label, 0, tx, ty, is_active ? t->foreground : (hovered ? t->foreground : t->text_muted));

        cur_x += tab_w + 4.0f;
    }

    return clicked_index;
}

bool ui_tab_item(const char* label, HttpMethod method, bool is_active, bool is_dirty, bool has_close, bool* out_close_clicked, Rect rect) {
    Theme* t = theme_get_active();
    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);
    bool clicked = hovered && g_ui.mouse_clicked[0];

    Color bg = is_active ? t->background : (hovered ? t->secondary_hover : t->tabbar_bg);
    r_draw_rect(rect, bg);
    r_draw_line((Vec2){rect.x + rect.w, rect.y}, (Vec2){rect.x + rect.w, rect.y + rect.h}, 1.0f, t->border);

    if (is_active) {
        r_draw_line((Vec2){rect.x, rect.y}, (Vec2){rect.x + rect.w, rect.y}, 2.0f, t->primary);
    }

    // Method Label
    const char* mstr = http_method_to_string(method);
    Color mcol = theme_get_method_color(mstr);
    float ty = rect.y + (rect.h - 14.0f) * 0.5f;
    font_draw_text(g_ui.font_bold, mstr, 0, rect.x + 8.0f, ty, mcol);

    float mw = font_measure_text(g_ui.font_bold, mstr, 0);
    float tx = rect.x + 8.0f + mw + 8.0f;

    // Title text
    r_push_clip(rect_make(tx, rect.y, rect.w - (tx - rect.x) - (has_close ? 24.0f : 8.0f), rect.h));
    font_draw_text(g_ui.font_ui, label, 0, tx, ty, is_active ? t->foreground : t->text_muted);
    r_pop_clip();

    // Close button '×'
    if (has_close) {
        Rect close_rect = rect_make(rect.x + rect.w - 20.0f, rect.y + (rect.h - 14.0f) * 0.5f, 14.0f, 14.0f);
        bool close_hovered = rect_contains(close_rect, g_ui.mouse_x, g_ui.mouse_y);
        if (close_hovered && g_ui.mouse_clicked[0]) {
            if (out_close_clicked) *out_close_clicked = true;
            clicked = false;
        }
        r_draw_icon(is_dirty ? ICON_DOT : ICON_CLOSE, close_rect, close_hovered ? t->danger : t->text_dim);
    }

    return clicked;
}

bool ui_dropdown(const char* id_str, const char** items, int item_count, int* selected_index, Rect rect) {
    Theme* t = theme_get_active();
    uint64_t id = ui_hash_id(id_str, 0);
    bool is_open = (g_ui.active_dropdown_id == id);
    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);

    if (hovered && g_ui.mouse_clicked[0]) {
        g_ui.active_dropdown_id = is_open ? 0 : id;
        is_open = !is_open;
    }

    Color bg = is_open ? t->secondary_active : (hovered ? t->secondary_hover : t->input_bg);
    Color bdr = is_open ? t->primary : t->input_border;
    r_draw_rect_rounded_bordered(rect, 4.0f, 1.0f, bdr, bg);

    int cur_sel = (selected_index && *selected_index >= 0 && *selected_index < item_count) ? *selected_index : 0;
    const char* current_label = (item_count > 0) ? items[cur_sel] : "";

    float tx = rect.x + 8.0f;
    float ty = rect.y + (rect.h - 14.0f) * 0.5f;
    Color txt_col = theme_get_method_color(current_label);
    font_draw_text(g_ui.font_bold, current_label, 0, tx, ty, txt_col);

    // Chevron Icon
    Rect ch_rect = rect_make(rect.x + rect.w - 18.0f, rect.y + (rect.h - 12.0f) * 0.5f, 12.0f, 12.0f);
    r_draw_icon(is_open ? ICON_CHEVRON_DOWN : ICON_CHEVRON_RIGHT, ch_rect, t->text_muted);

    bool changed = false;

    if (is_open) {
        float item_h = 26.0f;
        float menu_h = item_h * item_count;
        Rect menu_rect = rect_make(rect.x, rect.y + rect.h + 2.0f, rect.w, menu_h);

        r_draw_rect_rounded_bordered(menu_rect, 4.0f, 1.0f, t->border, t->panel_bg);

        for (int i = 0; i < item_count; i++) {
            Rect ir = rect_make(menu_rect.x + 1.0f, menu_rect.y + 1.0f + i * item_h, menu_rect.w - 2.0f, item_h);
            bool ih = rect_contains(ir, g_ui.mouse_x, g_ui.mouse_y);

            if (ih) {
                r_draw_rect_rounded(ir, 3.0f, t->secondary_hover);
                if (g_ui.mouse_clicked[0]) {
                    if (selected_index) *selected_index = i;
                    g_ui.active_dropdown_id = 0;
                    changed = true;
                }
            }

            Color icol = theme_get_method_color(items[i]);
            font_draw_text(g_ui.font_bold, items[i], 0, ir.x + 8.0f, ir.y + (item_h - 14.0f) * 0.5f, icol);
        }
    }

    return changed;
}

bool ui_tree_item(const char* id_str, const char* label, bool is_folder, bool* is_expanded,
                  HttpMethod method, int status_code, bool is_selected, bool* out_context_menu, Rect rect) {
    (void)id_str;
    Theme* t = theme_get_active();
    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);
    bool clicked = hovered && g_ui.mouse_clicked[0];
    bool r_clicked = hovered && g_ui.mouse_clicked[1];

    if (r_clicked && out_context_menu) {
        *out_context_menu = true;
    }

    Color bg = is_selected ? t->secondary_active : (hovered ? t->secondary_hover : color_rgba(0,0,0,0));
    if (bg.a > 0.01f) {
        r_draw_rect_rounded(rect, 4.0f, bg);
    }

    float cur_x = rect.x + 6.0f;
    float ty = rect.y + (rect.h - 14.0f) * 0.5f;

    if (is_folder) {
        Rect ch_rect = rect_make(cur_x, rect.y + (rect.h - 12.0f) * 0.5f, 12.0f, 12.0f);
        bool ch_hovered = rect_contains(ch_rect, g_ui.mouse_x, g_ui.mouse_y);
        if (ch_hovered && g_ui.mouse_clicked[0] && is_expanded) {
            *is_expanded = !*is_expanded;
            clicked = false;
        }
        r_draw_icon((is_expanded && *is_expanded) ? ICON_CHEVRON_DOWN : ICON_CHEVRON_RIGHT, ch_rect, t->text_muted);
        cur_x += 16.0f;

        Rect f_rect = rect_make(cur_x, rect.y + (rect.h - 14.0f) * 0.5f, 14.0f, 14.0f);
        r_draw_icon((is_expanded && *is_expanded) ? ICON_FOLDER_OPEN : ICON_FOLDER, f_rect, t->warning);
        cur_x += 20.0f;

        font_draw_text(g_ui.font_ui, label, 0, cur_x, ty, is_selected ? t->text_selected : (hovered ? t->foreground : t->foreground));
    } else {
        const char* mstr = http_method_to_string(method);
        Color mcol = theme_get_method_color(mstr);
        font_draw_text(g_ui.font_bold, mstr, 0, cur_x, ty, mcol);
        cur_x += 42.0f;

        font_draw_text(g_ui.font_ui, label, 0, cur_x, ty, is_selected ? t->text_selected : (hovered ? t->foreground : t->text_muted));

        if (status_code > 0) {
            char sbuf[16];
            snprintf(sbuf, sizeof(sbuf), "%d", status_code);
            Color scol = theme_get_status_color(status_code);
            float sw = font_measure_text(g_ui.font_bold, sbuf, 0);
            Rect badge_rect = rect_make(rect.x + rect.w - sw - 12.0f, rect.y + (rect.h - 18.0f) * 0.5f, sw + 8.0f, 18.0f);
            r_draw_rect_rounded(badge_rect, 3.0f, color_with_alpha(scol, 0.15f));
            font_draw_text(g_ui.font_bold, sbuf, 0, badge_rect.x + 4.0f, badge_rect.y + 2.0f, scol);
        }
    }

    return clicked;
}

bool ui_splitter(const char* id_str, float* split_pos, bool is_vertical, float min_a, float min_b, Rect container) {
    (void)id_str;
    Theme* t = theme_get_active();
    float thick = 4.0f;
    Rect handle_rect;

    if (is_vertical) {
        handle_rect = rect_make(container.x + *split_pos - thick * 0.5f, container.y, thick, container.h);
    } else {
        handle_rect = rect_make(container.x, container.y + *split_pos - thick * 0.5f, container.w, thick);
    }

    bool hovered = rect_contains(handle_rect, g_ui.mouse_x, g_ui.mouse_y);
    static bool s_dragging = false;

    if (hovered && g_ui.mouse_down[0]) s_dragging = true;
    if (!g_ui.mouse_down[0]) s_dragging = false;

    if (s_dragging) {
        if (is_vertical) {
            *split_pos = g_ui.mouse_x - container.x;
            if (*split_pos < min_a) *split_pos = min_a;
            if (*split_pos > container.w - min_b) *split_pos = container.w - min_b;
        } else {
            *split_pos = g_ui.mouse_y - container.y;
            if (*split_pos < min_a) *split_pos = min_a;
            if (*split_pos > container.h - min_b) *split_pos = container.h - min_b;
        }
    }

    Color line_col = s_dragging ? t->primary : (hovered ? t->secondary_active : t->border);
    if (is_vertical) {
        r_draw_rect(rect_make(container.x + *split_pos, container.y, 1.0f, container.h), line_col);
    } else {
        r_draw_rect(rect_make(container.x, container.y + *split_pos, container.w, 1.0f), line_col);
    }

    return s_dragging;
}

bool ui_kv_editor(const char* id_str, KeyValue** items_ptr, size_t* count_ptr, Rect rect, float* scroll_y) {
    (void)id_str;
    (void)scroll_y;
    Theme* t = theme_get_active();
    if (!items_ptr || !count_ptr) return false;

    float row_h = 28.0f;
    float cur_y = rect.y;

    // Header row
    r_draw_rect(rect_make(rect.x, cur_y, rect.w, 24.0f), t->panel_bg);
    font_draw_text(g_ui.font_bold, "Key", 0, rect.x + 36.0f, cur_y + 5.0f, t->text_muted);
    font_draw_text(g_ui.font_bold, "Value", 0, rect.x + rect.w * 0.45f + 8.0f, cur_y + 5.0f, t->text_muted);
    cur_y += 26.0f;

    bool changed = false;
    size_t count = *count_ptr;
    KeyValue* items = *items_ptr;

    for (size_t i = 0; i < count; i++) {
        Rect row_rect = rect_make(rect.x, cur_y, rect.w, row_h);

        // Checkbox
        Rect cb_rect = rect_make(row_rect.x + 4.0f, row_rect.y, 24.0f, row_h);
        if (ui_checkbox(NULL, &items[i].enabled, cb_rect)) changed = true;

        // Key Input
        float key_w = rect.w * 0.4f;
        Rect key_rect = rect_make(row_rect.x + 32.0f, row_rect.y + 2.0f, key_w, row_h - 4.0f);
        char key_id[64];
        snprintf(key_id, sizeof(key_id), "%s_k_%zu", id_str, i);
        if (ui_input_text(key_id, items[i].key, sizeof(items[i].key), "Key", key_rect)) changed = true;

        // Value Input
        float val_w = rect.w - key_w - 74.0f;
        Rect val_rect = rect_make(key_rect.x + key_w + 6.0f, row_rect.y + 2.0f, val_w, row_h - 4.0f);
        char val_id[64];
        snprintf(val_id, sizeof(val_id), "%s_v_%zu", id_str, i);
        if (ui_input_text(val_id, items[i].value, sizeof(items[i].value), "Value", val_rect)) changed = true;

        // Delete Row '×'
        Rect del_rect = rect_make(val_rect.x + val_w + 6.0f, row_rect.y + 4.0f, 20.0f, 20.0f);
        if (ui_button_ghost("×", del_rect)) {
            kv_array_remove(items_ptr, count_ptr, i);
            changed = true;
            break;
        }

        cur_y += row_h + 2.0f;
    }

    // '+ Add' Button
    Rect add_rect = rect_make(rect.x + 4.0f, cur_y + 4.0f, 64.0f, 24.0f);
    if (ui_button_ghost("+ Add", add_rect)) {
        kv_array_add(items_ptr, count_ptr, "", "", true);
        changed = true;
    }

    return changed;
}

bool ui_data_table(const char* id_str, const char** columns, size_t col_count,
                   char*** rows, size_t row_count, const char* filter_text,
                   int* selected_row, int* selected_col, Rect rect, float* scroll_y) {
    (void)id_str;
    (void)selected_col;
    Theme* t = theme_get_active();
    if (!columns || col_count == 0 || !rows) return false;

    float row_h = 24.0f;
    float header_h = 26.0f;

    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);
    if (hovered && scroll_y) {
        *scroll_y -= g_ui.scroll_y * 24.0f;
        if (*scroll_y < 0.0f) *scroll_y = 0.0f;
    }

    r_draw_rect(rect, t->background);

    float col_w = (rect.w - 10.0f) / (float)col_count;
    if (col_w < 110.0f) col_w = 110.0f;

    // Header
    Rect h_rect = rect_make(rect.x, rect.y, rect.w, header_h);
    r_draw_rect(h_rect, t->panel_bg);
    r_draw_line((Vec2){rect.x, rect.y + header_h}, (Vec2){rect.x + rect.w, rect.y + header_h}, 1.0f, t->border);

    for (size_t c = 0; c < col_count; c++) {
        Rect cell = rect_make(rect.x + c * col_w, rect.y, col_w, header_h);
        r_draw_line((Vec2){cell.x + cell.w, cell.y}, (Vec2){cell.x + cell.w, cell.y + cell.h}, 1.0f, t->border_subtle);
        font_draw_text(g_ui.font_bold, columns[c], 0, cell.x + 8.0f, cell.y + 6.0f, t->foreground);
    }

    // Rows
    r_push_clip(rect_make(rect.x, rect.y + header_h, rect.w, rect.h - header_h));
    float cur_y = rect.y + header_h - (scroll_y ? *scroll_y : 0.0f);

    for (size_t r = 0; r < row_count; r++) {
        if (filter_text && *filter_text) {
            bool matches = false;
            for (size_t c = 0; c < col_count; c++) {
                if (strcasestr(rows[r][c], filter_text)) {
                    matches = true;
                    break;
                }
            }
            if (!matches) continue;
        }

        if (cur_y + row_h >= rect.y && cur_y <= rect.y + rect.h) {
            Rect row_rect = rect_make(rect.x, cur_y, rect.w, row_h);
            bool row_hovered = rect_contains(row_rect, g_ui.mouse_x, g_ui.mouse_y);
            bool is_sel = (selected_row && *selected_row == (int)r);

            Color row_bg = is_sel ? t->selection_bg : (row_hovered ? t->secondary_hover : ((r % 2 == 1) ? t->panel_bg : t->background));
            r_draw_rect(row_rect, row_bg);
            r_draw_line((Vec2){rect.x, cur_y + row_h}, (Vec2){rect.x + rect.w, cur_y + row_h}, 1.0f, t->border_subtle);

            if (row_hovered && g_ui.mouse_clicked[0] && selected_row) {
                *selected_row = (int)r;
            }

            for (size_t c = 0; c < col_count; c++) {
                Rect cell = rect_make(rect.x + c * col_w, cur_y, col_w, row_h);
                r_draw_line((Vec2){cell.x + cell.w, cell.y}, (Vec2){cell.x + cell.w, cell.y + cell.h}, 1.0f, t->border_subtle);

                r_push_clip(rect_make(cell.x + 2.0f, cell.y, cell.w - 4.0f, cell.h));
                font_draw_text(g_ui.font_ui, rows[r][c], 0, cell.x + 8.0f, cell.y + 5.0f, is_sel ? t->text_selected : t->foreground);
                r_pop_clip();
            }
        }

        cur_y += row_h;
    }

    r_pop_clip();
    return false;
}

bool ui_modal_begin(const char* title, float w, float h, Rect* out_body_rect) {
    Theme* t = theme_get_active();
    s_modal_active = true;

    r_draw_rect(rect_make(0, 0, (float)g_ui.window_width, (float)g_ui.window_height), color_rgba(0, 0, 0, 0.65f));

    float mx = ((float)g_ui.window_width - w) * 0.5f;
    float my = ((float)g_ui.window_height - h) * 0.5f;
    s_modal_rect = rect_make(mx, my, w, h);

    r_draw_rect_rounded_bordered(s_modal_rect, 6.0f, 1.0f, t->border, t->panel_bg);

    r_draw_line((Vec2){mx, my + 40.0f}, (Vec2){mx + w, my + 40.0f}, 1.0f, t->border);
    font_draw_text(g_ui.font_bold, title, 0, mx + 16.0f, my + 12.0f, t->foreground);

    Rect close_btn = rect_make(mx + w - 32.0f, my + 10.0f, 20.0f, 20.0f);
    if (ui_button_ghost("×", close_btn) || (g_ui.key_pressed == GLFW_KEY_ESCAPE)) {
        return false;
    }

    if (out_body_rect) {
        *out_body_rect = rect_make(mx + 16.0f, my + 50.0f, w - 32.0f, h - 66.0f);
    }

    return true;
}

void ui_modal_end(void) {
    s_modal_active = false;
}

bool ui_context_menu_begin(const char* id_str, float w, float* out_y) {
    Theme* t = theme_get_active();
    uint64_t id = ui_hash_id(id_str, 0);
    s_cm_active_id = id;
    s_cm_rect = rect_make(g_ui.mouse_x, g_ui.mouse_y, w, 160.0f);
    r_draw_rect_rounded_bordered(s_cm_rect, 4.0f, 1.0f, t->border, t->panel_bg);
    if (out_y) *out_y = s_cm_rect.y + 4.0f;
    return true;
}

bool ui_context_menu_item(IconType icon, const char* label, float w, float* y_cursor) {
    Theme* t = theme_get_active();
    float item_h = 24.0f;
    Rect r = rect_make(s_cm_rect.x + 4.0f, *y_cursor, w - 8.0f, item_h);
    bool hovered = rect_contains(r, g_ui.mouse_x, g_ui.mouse_y);
    bool clicked = hovered && g_ui.mouse_clicked[0];

    if (hovered) {
        r_draw_rect_rounded(r, 3.0f, t->secondary_hover);
    }

    float cur_x = r.x + 6.0f;
    if (icon != ICON_NONE) {
        r_draw_icon(icon, rect_make(cur_x, r.y + (item_h - 12.0f) * 0.5f, 12.0f, 12.0f), hovered ? t->foreground : t->text_muted);
        cur_x += 16.0f;
    }

    font_draw_text(g_ui.font_ui, label, 0, cur_x, r.y + 5.0f, hovered ? t->foreground : t->text_muted);
    *y_cursor += item_h + 2.0f;

    return clicked;
}

void ui_context_menu_end(void) {
    s_cm_active_id = 0;
}

#include "ui/ui.h"
#include "icons/icon.h"
#include "gfx/gfx.h"
#include "json/json_utils.h"
#include "ui/spring.h"
#include "ui/editbuf.h"
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

// Deferred Dropdown Overlay State (rendered on top of all panels/dialogs without clipping)
typedef struct {
    uint64_t id;
    char items[32][64];
    int item_count;
    int* selected_index;
    Rect anchor_rect;
    bool is_open;
    bool just_opened;
    bool changed;
} DropdownOverlayState;

static DropdownOverlayState s_dropdown_overlay = {0};

// UILayout Auto-Layout Implementation
UILayout ui_layout_row(Rect container, float gap, float pad_x, float pad_y) {
    UILayout l;
    l.container = container;
    l.gap = gap;
    l.pad_x = pad_x;
    l.pad_y = pad_y;
    l.cur_x = container.x + pad_x;
    l.cur_y = container.y + pad_y;
    l.max_x = container.x + container.w - pad_x;
    l.max_y = container.y + container.h - pad_y;
    l.is_row = true;
    return l;
}

UILayout ui_layout_col(Rect container, float gap, float pad_x, float pad_y) {
    UILayout l;
    l.container = container;
    l.gap = gap;
    l.pad_x = pad_x;
    l.pad_y = pad_y;
    l.cur_x = container.x + pad_x;
    l.cur_y = container.y + pad_y;
    l.max_x = container.x + container.w - pad_x;
    l.max_y = container.y + container.h - pad_y;
    l.is_row = false;
    return l;
}

Rect ui_layout_item_fixed(UILayout* l, float main_size, float cross_size) {
    if (l->is_row) {
        float h = (cross_size > 0.0f) ? cross_size : (l->max_y - l->cur_y);
        float w = main_size;
        if (l->cur_x + w > l->max_x) {
            w = l->max_x - l->cur_x;
            if (w < 0.0f) w = 0.0f;
        }
        Rect r = rect_make(l->cur_x, l->cur_y + (l->max_y - l->cur_y - h) * 0.5f, w, h);
        l->cur_x += main_size + l->gap;
        return r;
    } else {
        float w = (cross_size > 0.0f) ? cross_size : (l->max_x - l->cur_x);
        float h = main_size;
        if (l->cur_y + h > l->max_y) {
            h = l->max_y - l->cur_y;
            if (h < 0.0f) h = 0.0f;
        }
        Rect r = rect_make(l->cur_x + (l->max_x - l->cur_x - w) * 0.5f, l->cur_y, w, h);
        l->cur_y += main_size + l->gap;
        return r;
    }
}

Rect ui_layout_item_flex(UILayout* l, float flex_w, float min_w, float cross_size) {
    (void)flex_w;
    if (l->is_row) {
        float h = (cross_size > 0.0f) ? cross_size : (l->max_y - l->cur_y);
        float avail = l->max_x - l->cur_x;
        if (avail < min_w) avail = min_w;
        Rect r = rect_make(l->cur_x, l->cur_y + (l->max_y - l->cur_y - h) * 0.5f, avail > 0.0f ? avail : 0.0f, h);
        l->cur_x += avail + l->gap;
        return r;
    } else {
        float w = (cross_size > 0.0f) ? cross_size : (l->max_x - l->cur_x);
        float avail = l->max_y - l->cur_y;
        if (avail < min_w) avail = min_w;
        Rect r = rect_make(l->cur_x + (l->max_x - l->cur_x - w) * 0.5f, l->cur_y, w, avail > 0.0f ? avail : 0.0f);
        l->cur_y += avail + l->gap;
        return r;
    }
}

Rect ui_layout_item_right(UILayout* l, float width, float height) {
    float h = (height > 0.0f) ? height : (l->max_y - l->cur_y);
    float x = l->max_x - width;
    Rect r = rect_make(x, l->cur_y + (l->max_y - l->cur_y - h) * 0.5f, width, h);
    l->max_x -= (width + l->gap);
    return r;
}

bool ui_layout_can_fit(const UILayout* l, float required_size) {
    if (l->is_row) {
        return (l->cur_x + required_size <= l->max_x);
    } else {
        return (l->cur_y + required_size <= l->max_y);
    }
}

float ui_layout_remaining_space(const UILayout* l) {
    if (l->is_row) {
        float rem = l->max_x - l->cur_x;
        return rem > 0.0f ? rem : 0.0f;
    } else {
        float rem = l->max_y - l->cur_y;
        return rem > 0.0f ? rem : 0.0f;
    }
}

// FredBuffer Editor Search State
typedef struct {
    bool is_open;
    char query[128];
    bool case_sensitive;
    int active_match_idx;
    TextRange* matches;
    size_t match_count;
    bool matches_dirty;
} EditorSearchState;

// FredBuffer Editor Slot Cache for persistent undo/redo & gap-buffer performance
typedef struct {
    uint64_t id;
    FredBuffer* buf;
    const void* last_source_ptr;
    size_t last_source_len;
    char* cached_text;
    size_t cached_text_len;
    TextLayout layout;
    float last_wrap_w;
    bool layout_valid;
    JsonToken* cached_tokens;
    size_t token_count;
    bool tokens_valid;
    EditorSearchState search;
} EditorBufferSlot;

#define MAX_EDITOR_SLOTS 16
static EditorBufferSlot s_editor_slots[MAX_EDITOR_SLOTS] = {0};

static EditorBufferSlot* get_or_create_editor_slot(uint64_t id, const char* text, const void* source_ptr) {
    size_t tlen = text ? strlen(text) : 0;

    for (int i = 0; i < MAX_EDITOR_SLOTS; i++) {
        if (s_editor_slots[i].id == id && s_editor_slots[i].buf) {
            EditorBufferSlot* slot = &s_editor_slots[i];
            // Detect if the external source changed
            if (slot->last_source_ptr != source_ptr || slot->last_source_len != tlen ||
                (slot->cached_text && text && strcmp(slot->cached_text, text) != 0)) {
                fredbuf_replace_all(slot->buf, text ? text : "");
                slot->last_source_ptr = source_ptr;
                slot->last_source_len = tlen;
                if (slot->cached_text) {
                    free(slot->cached_text);
                    slot->cached_text = NULL;
                }
                slot->cached_text = fredbuf_to_string(slot->buf);
                slot->cached_text_len = fredbuf_length(slot->buf);
                if (slot->layout_valid) {
                    font_layout_free(&slot->layout);
                    slot->layout_valid = false;
                }
                if (slot->cached_tokens) {
                    free(slot->cached_tokens);
                    slot->cached_tokens = NULL;
                    slot->tokens_valid = false;
                }
                slot->search.matches_dirty = true;
            }
            return slot;
        }
    }

    for (int i = 0; i < MAX_EDITOR_SLOTS; i++) {
        if (s_editor_slots[i].id == 0) {
            EditorBufferSlot* slot = &s_editor_slots[i];
            slot->id = id;
            slot->buf = fredbuf_create_from_string(text ? text : "");
            slot->last_source_ptr = source_ptr;
            slot->last_source_len = tlen;
            slot->cached_text = fredbuf_to_string(slot->buf);
            slot->cached_text_len = fredbuf_length(slot->buf);
            slot->layout_valid = false;
            slot->tokens_valid = false;
            slot->search.matches_dirty = true;
            return slot;
        }
    }

    // Evict slot 0
    EditorBufferSlot* slot0 = &s_editor_slots[0];
    if (slot0->buf) fredbuf_free(slot0->buf);
    if (slot0->cached_text) free(slot0->cached_text);
    if (slot0->layout_valid) font_layout_free(&slot0->layout);
    if (slot0->cached_tokens) free(slot0->cached_tokens);
    if (slot0->search.matches) free(slot0->search.matches);
    memset(slot0, 0, sizeof(EditorBufferSlot));

    slot0->id = id;
    slot0->buf = fredbuf_create_from_string(text ? text : "");
    slot0->last_source_ptr = source_ptr;
    slot0->last_source_len = tlen;
    slot0->cached_text = fredbuf_to_string(slot0->buf);
    slot0->cached_text_len = fredbuf_length(slot0->buf);
    slot0->search.matches_dirty = true;
    return slot0;
}

uint64_t ui_hash_id(const char* str, uint64_t seed) {
    if (!str) return seed;
    uint64_t hash = seed ^ 14695981039346656037ULL;
    while (*str) {
        hash ^= (uint8_t)*str++;
        hash *= 1099511628211ULL;
    }
    return hash;
}

void ui_init(int window_w, int window_h, UiFontSpec fonts) {
    icon_system_init();
    g_ui.window_width = window_w;
    g_ui.window_height = window_h;

    // Load crisp FreeType & HarfBuzz fonts
    g_ui.font_ui = font_load_ui_family(fonts.family_ui, fonts.size_ui > 0.0f ? fonts.size_ui : 15.0f);
    g_ui.font_mono = font_load_mono_family(fonts.family_mono, fonts.size_mono > 0.0f ? fonts.size_mono : 14.0f);
    g_ui.font_bold = font_load_bold_family(fonts.family_ui, fonts.size_bold > 0.0f ? fonts.size_bold : 15.0f);

    theme_init();
}

void ui_shutdown(void) {
    icon_system_shutdown();
    if (g_ui.font_ui) font_free(g_ui.font_ui);
    if (g_ui.font_mono) font_free(g_ui.font_mono);
    if (g_ui.font_bold) font_free(g_ui.font_bold);
    for (int i = 0; i < MAX_EDITOR_SLOTS; i++) {
        if (s_editor_slots[i].buf) {
            fredbuf_free(s_editor_slots[i].buf);
            s_editor_slots[i].buf = NULL;
        }
        if (s_editor_slots[i].cached_text) {
            free(s_editor_slots[i].cached_text);
            s_editor_slots[i].cached_text = NULL;
        }
        if (s_editor_slots[i].layout_valid) {
            font_layout_free(&s_editor_slots[i].layout);
            s_editor_slots[i].layout_valid = false;
        }
        if (s_editor_slots[i].cached_tokens) {
            free(s_editor_slots[i].cached_tokens);
            s_editor_slots[i].cached_tokens = NULL;
            s_editor_slots[i].tokens_valid = false;
        }
        if (s_editor_slots[i].search.matches) {
            free(s_editor_slots[i].search.matches);
            s_editor_slots[i].search.matches = NULL;
        }
        s_editor_slots[i].id = 0;
    }
}

void ui_begin_frame(float dt) {
    // Advances the shaped-run cache generation so stale entries can be reused.
    font_begin_frame();

    g_ui.dt = dt;
    s_cursor_blink += dt * 3.0f;
    s_has_tooltip = false;
    s_tooltip_text[0] = '\0';
    g_ui.hover_id = 0;
    if (!g_ui.mouse_down[0]) {
        g_ui.is_selecting = false;
    }
    if (g_ui.scroll_y != 0.0f || g_ui.scroll_x != 0.0f) {
        g_ui.is_selecting = false;
    }
}

static void ui_render_dropdown_overlay(void) {
    if (!s_dropdown_overlay.is_open || s_dropdown_overlay.item_count <= 0) return;
    Theme* t = theme_get_active();
    float item_h = 28.0f;
    float menu_h = item_h * s_dropdown_overlay.item_count + 6.0f;
    float menu_w = s_dropdown_overlay.anchor_rect.w;
    if (menu_w < 110.0f) menu_w = 110.0f;

    float menu_x = s_dropdown_overlay.anchor_rect.x;
    float menu_y = s_dropdown_overlay.anchor_rect.y + s_dropdown_overlay.anchor_rect.h + 2.0f;

    // Flip upwards if exceeding screen bottom
    if (menu_y + menu_h > (float)g_ui.window_height - 10.0f) {
        menu_y = s_dropdown_overlay.anchor_rect.y - menu_h - 2.0f;
    }

    Rect menu_rect = rect_make(menu_x, menu_y, menu_w, menu_h);

    // 1. Elevation Drop Shadow
    r_draw_rect_rounded(rect_make(menu_rect.x + 2.0f, menu_rect.y + 4.0f, menu_rect.w, menu_rect.h), 6.0f, color_rgba(0, 0, 0, 0.45f));

    // 2. Solid Background & Border
    r_draw_rect_rounded_bordered(menu_rect, 6.0f, 1.0f, t->border, t->panel_bg);

    float fh_bold = font_get_height(g_ui.font_bold);

    for (int i = 0; i < s_dropdown_overlay.item_count; i++) {
        Rect ir = rect_make(menu_rect.x + 3.0f, menu_rect.y + 3.0f + i * item_h, menu_rect.w - 6.0f, item_h);
        bool ih = rect_contains(ir, g_ui.mouse_x, g_ui.mouse_y);

        if (ih) {
            r_draw_rect_rounded(ir, 4.0f, t->secondary_hover);
            if (g_ui.mouse_clicked[0]) {
                if (s_dropdown_overlay.selected_index) {
                    *s_dropdown_overlay.selected_index = i;
                }
                s_dropdown_overlay.is_open = false;
                s_dropdown_overlay.changed = true;
            }
        }

        const char* label = s_dropdown_overlay.items[i];
        Color icol = theme_get_method_color(label);
        font_draw_text(g_ui.font_bold, label, 0, ir.x + 8.0f, ir.y + (item_h - fh_bold) * 0.5f, icol);
    }

    // Dismiss when clicking outside
    if (g_ui.mouse_clicked[0] && !s_dropdown_overlay.just_opened) {
        if (!rect_contains(menu_rect, g_ui.mouse_x, g_ui.mouse_y) &&
            !rect_contains(s_dropdown_overlay.anchor_rect, g_ui.mouse_x, g_ui.mouse_y)) {
            s_dropdown_overlay.is_open = false;
        }
    }
    s_dropdown_overlay.just_opened = false;
}

void ui_end_frame(void) {
    Theme* t = theme_get_active();

    // 1. Render active dropdown overlay (always on top of all panels/dialogs, unclipped)
    ui_render_dropdown_overlay();

    // 2. Render tooltip if active
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
        float fh = font_get_height(g_ui.font_bold);
        float tw = font_measure_text(g_ui.font_bold, label, 0);
        float tx = rect.x + (rect.w - tw) * 0.5f;
        float ty = rect.y + (rect.h - fh) * 0.5f;
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
        float fh = font_get_height(g_ui.font_ui);
        float tw = font_measure_text(g_ui.font_ui, label, 0);
        float tx = rect.x + (rect.w - tw) * 0.5f;
        float ty = rect.y + (rect.h - fh) * 0.5f;
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
        float fh = font_get_height(g_ui.font_bold);
        float tw = font_measure_text(g_ui.font_bold, label, 0);
        float tx = rect.x + (rect.w - tw) * 0.5f;
        float ty = rect.y + (rect.h - fh) * 0.5f;
        font_draw_text(g_ui.font_bold, label, 0, tx, ty, color_hex(0xFFFFFF));
    }

    return clicked;
}

bool ui_button_primary_icon(IconType icon, const char* label, Rect rect) {
    Theme* t = theme_get_active();
    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);
    bool clicked = hovered && g_ui.mouse_clicked[0];

    Color bg = hovered ? t->primary_hover : t->primary;
    Color txt = t->primary_text;

    r_draw_rect_rounded(rect, 4.0f, bg);

    float isz = 14.0f;
    float fh = font_get_height(g_ui.font_bold);
    float tw = label ? font_measure_text(g_ui.font_bold, label, 0) : 0.0f;
    float total_w = (icon != ICON_NONE ? isz + 6.0f : 0.0f) + tw;
    float cur_x = rect.x + (rect.w - total_w) * 0.5f;

    if (icon != ICON_NONE) {
        Rect ir = rect_make(cur_x, rect.y + (rect.h - isz) * 0.5f, isz, isz);
        icon_draw(icon, ir, txt);
        cur_x += isz + 6.0f;
    }

    if (label) {
        float ty = rect.y + (rect.h - fh) * 0.5f;
        font_draw_text(g_ui.font_bold, label, 0, cur_x, ty, txt);
    }

    return clicked;
}

bool ui_button_danger_icon(IconType icon, const char* label, Rect rect) {
    Theme* t = theme_get_active();
    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);
    bool clicked = hovered && g_ui.mouse_clicked[0];

    Color bg = hovered ? t->danger_hover : t->danger;
    Color txt = color_hex(0xFFFFFF);

    r_draw_rect_rounded(rect, 4.0f, bg);

    float isz = 14.0f;
    float fh = font_get_height(g_ui.font_bold);
    float tw = label ? font_measure_text(g_ui.font_bold, label, 0) : 0.0f;
    float total_w = (icon != ICON_NONE ? isz + 6.0f : 0.0f) + tw;
    float cur_x = rect.x + (rect.w - total_w) * 0.5f;

    if (icon != ICON_NONE) {
        Rect ir = rect_make(cur_x, rect.y + (rect.h - isz) * 0.5f, isz, isz);
        icon_draw(icon, ir, txt);
        cur_x += isz + 6.0f;
    }

    if (label) {
        float ty = rect.y + (rect.h - fh) * 0.5f;
        font_draw_text(g_ui.font_bold, label, 0, cur_x, ty, txt);
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

    float isz = (rect.h < 26.0f) ? 13.0f : 15.0f;
    float iy = rect.y + (rect.h - isz) * 0.5f;
    float fh = font_get_height(g_ui.font_ui);

    if (icon != ICON_NONE && label && *label) {
        float tw = font_measure_text(g_ui.font_ui, label, 0);
        float total_w = isz + 6.0f + tw;
        float cur_x = rect.x + (rect.w - total_w) * 0.5f;
        if (cur_x < rect.x + 6.0f) cur_x = rect.x + 6.0f;

        Rect icon_rect = rect_make(cur_x, iy, isz, isz);
        icon_draw(icon, icon_rect, col);
        cur_x += isz + 6.0f;

        float ty = rect.y + (rect.h - fh) * 0.5f;
        font_draw_text(g_ui.font_ui, label, 0, cur_x, ty, col);
    } else if (icon != ICON_NONE) {
        float ix = rect.x + (rect.w - isz) * 0.5f;
        Rect icon_rect = rect_make(ix, iy, isz, isz);
        icon_draw(icon, icon_rect, col);
    } else if (label && *label) {
        float tw = font_measure_text(g_ui.font_ui, label, 0);
        float tx = rect.x + (rect.w - tw) * 0.5f;
        float ty = rect.y + (rect.h - fh) * 0.5f;
        font_draw_text(g_ui.font_ui, label, 0, tx, ty, col);
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
        icon_draw(ICON_CHECK, box, t->primary_text);
    }

    if (label) {
        float fh = font_get_height(g_ui.font_ui);
        float tx = box.x + box.w + 6.0f;
        float ty = rect.y + (rect.h - fh) * 0.5f;
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

static void find_word_bounds(const char* str, size_t len, size_t pos, size_t* out_start, size_t* out_end) {
    if (!str || len == 0) {
        if (out_start) *out_start = 0;
        if (out_end) *out_end = 0;
        return;
    }
    if (pos > len) pos = len;

    size_t start = pos;
    size_t end = pos;

    if (pos < len && (isalnum((unsigned char)str[pos]) || str[pos] == '_')) {
        while (start > 0 && (isalnum((unsigned char)str[start - 1]) || str[start - 1] == '_')) start--;
        while (end < len && (isalnum((unsigned char)str[end]) || str[end] == '_')) end++;
    } else if (pos > 0 && (isalnum((unsigned char)str[pos - 1]) || str[pos - 1] == '_')) {
        start = pos - 1;
        while (start > 0 && (isalnum((unsigned char)str[start - 1]) || str[start - 1] == '_')) start--;
        end = pos;
        while (end < len && (isalnum((unsigned char)str[end]) || str[end] == '_')) end++;
    } else {
        while (start > 0 && !isalnum((unsigned char)str[start - 1]) && !isspace((unsigned char)str[start - 1])) start--;
        while (end < len && !isalnum((unsigned char)str[end]) && !isspace((unsigned char)str[end])) end++;
    }

    if (out_start) *out_start = start;
    if (out_end) *out_end = end;
}

static uint64_t s_input_active_id = 0;
static size_t s_input_cursor = 0;
static size_t s_input_anchor = 0;
static bool s_input_is_dragging = false;
static CaretSpring s_input_caret = {0};
static uint64_t s_input_caret_id = 0;
static float s_input_scroll_x = 0.0f;
static uint64_t s_input_scroll_id = 0;

bool ui_input_text_icon(const char* id_str, IconType icon, char* buffer, size_t buffer_size, const char* placeholder, Rect rect) {
    Theme* t = theme_get_active();
    uint64_t id = ui_hash_id(id_str, 0);
    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);
    bool is_focused = (g_ui.focus_id == id);

    size_t len = buffer ? strlen(buffer) : 0;
    float fh = font_get_height(g_ui.font_ui);

    if (s_input_scroll_id != id) {
        s_input_scroll_x = 0.0f;
        s_input_scroll_id = id;
    }

    float clip_left = (icon != ICON_NONE) ? 28.0f : 8.0f;
    float avail_w = rect.w - clip_left - 8.0f;
    if (avail_w < 20.0f) avail_w = 20.0f;

    float cur_cursor_px = (s_input_cursor > 0 && buffer) ? font_measure_text(g_ui.font_ui, buffer, s_input_cursor) : 0.0f;
    if (is_focused) {
        if (cur_cursor_px - s_input_scroll_x > avail_w - 16.0f) {
            s_input_scroll_x = cur_cursor_px - avail_w + 16.0f;
        } else if (cur_cursor_px - s_input_scroll_x < 12.0f) {
            s_input_scroll_x = cur_cursor_px - 12.0f;
        }
        if (s_input_scroll_x < 0.0f) s_input_scroll_x = 0.0f;
    } else {
        s_input_scroll_x = 0.0f;
    }

    float base_tx = (icon != ICON_NONE) ? (rect.x + 28.0f) : (rect.x + 8.0f);
    float tx = base_tx - s_input_scroll_x;
    float ty = rect.y + (rect.h - fh) * 0.5f;

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

        if (g_ui.mouse_click_count == 2) {
            // Double click: select word
            find_word_bounds(buffer, len, clicked_pos, &s_input_anchor, &s_input_cursor);
            s_input_is_dragging = false;
        } else if (g_ui.mouse_click_count == 3) {
            // Triple click: select all
            s_input_anchor = 0;
            s_input_cursor = len;
            s_input_is_dragging = false;
        } else {
            // Single click
            if (g_ui.shift_down) {
                s_input_cursor = clicked_pos;
            } else {
                s_input_cursor = clicked_pos;
                s_input_anchor = clicked_pos;
            }
            s_input_is_dragging = true;
        }

        if (s_input_caret_id != id) {
            float cur_px = tx + (s_input_cursor > 0 ? font_measure_text(g_ui.font_ui, buffer, s_input_cursor) : 0.0f);
            s_input_caret = caret_spring_make(cur_px, ty, fh);
            s_input_caret_id = id;
        }
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
                caret_spring_notify_typing(&s_input_caret);
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
                    caret_spring_notify_typing(&s_input_caret);
                }
            }
        } else if (g_ui.key_pressed == GLFW_KEY_BACKSPACE) {
            if (has_sel) {
                memmove(buffer + s_min, buffer + s_max, len - s_max + 1);
                s_input_cursor = s_input_anchor = s_min;
                changed = true;
                len = strlen(buffer);
                caret_spring_notify_typing(&s_input_caret);
            } else if (s_input_cursor > 0) {
                memmove(buffer + s_input_cursor - 1, buffer + s_input_cursor, len - s_input_cursor + 1);
                s_input_cursor--;
                s_input_anchor = s_input_cursor;
                changed = true;
                len = strlen(buffer);
                caret_spring_notify_typing(&s_input_caret);
            }
        } else if (g_ui.key_pressed == GLFW_KEY_DELETE) {
            if (has_sel) {
                memmove(buffer + s_min, buffer + s_max, len - s_max + 1);
                s_input_cursor = s_input_anchor = s_min;
                changed = true;
                len = strlen(buffer);
                caret_spring_notify_typing(&s_input_caret);
            } else if (s_input_cursor < len) {
                memmove(buffer + s_input_cursor, buffer + s_input_cursor + 1, len - s_input_cursor);
                changed = true;
                len = strlen(buffer);
                caret_spring_notify_typing(&s_input_caret);
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
                caret_spring_notify_typing(&s_input_caret);
            }
        }
    }

    Color bg = t->input_bg;
    Color bdr = is_focused ? t->input_border_focus : (hovered ? t->border : t->input_border);

    r_draw_rect_rounded_bordered(rect, 4.0f, 1.0f, bdr, bg);

    if (icon != ICON_NONE) {
        float isz = 13.0f;
        Rect ir = rect_make(rect.x + 8.0f, rect.y + (rect.h - isz) * 0.5f, isz, isz);
        icon_draw(icon, ir, is_focused ? t->primary : (hovered ? t->foreground : t->text_muted));
    }

    r_push_clip(rect_make(rect.x + clip_left, rect.y + 1.0f, rect.w - clip_left - 4.0f, rect.h - 2.0f));

    if (buffer && buffer[0]) {
        size_t s_min = (is_focused && s_input_active_id == id) ? (s_input_cursor < s_input_anchor ? s_input_cursor : s_input_anchor) : 0;
        size_t s_max = (is_focused && s_input_active_id == id) ? (s_input_cursor > s_input_anchor ? s_input_cursor : s_input_anchor) : 0;
        bool has_sel = (s_min != s_max);

        // Draw selection background
        if (has_sel) {
            float sx0 = tx + (s_min > 0 ? font_measure_text(g_ui.font_ui, buffer, s_min) : 0.0f);
            float sx1 = tx + (s_max > 0 ? font_measure_text(g_ui.font_ui, buffer, s_max) : 0.0f);
            if (sx1 > sx0) {
                r_draw_rect_rounded(rect_make(sx0, ty, sx1 - sx0, fh), 2.0f, t->selection_bg);
            }
        }

        const char* p = buffer;
        float cx = tx;
        while (*p) {
            if (p[0] == '{' && p[1] == '{') {
                const char* end = strstr(p, "}}");
                if (end) {
                    size_t var_len = (end + 2) - p;
                    float vw = (var_len > 0) ? font_measure_text(g_ui.font_ui, p, var_len) : 0.0f;
                    r_draw_rect_rounded(rect_make(cx - 2.0f, ty, vw + 4.0f, fh), 3.0f, color_with_alpha(t->warning, 0.2f));
                    font_draw_text(g_ui.font_ui, p, var_len, cx, ty, t->warning);
                    cx += vw;
                    p = end + 2;
                    continue;
                }
            }
            // Non-variable chunk
            const char* next_var = strstr(p, "{{");
            size_t chunk_len = next_var ? (size_t)(next_var - p) : strlen(p);
            float cw = (chunk_len > 0) ? font_measure_text(g_ui.font_ui, p, chunk_len) : 0.0f;
            if (chunk_len > 0) {
                font_draw_text(g_ui.font_ui, p, chunk_len, cx, ty, t->foreground);
            }
            cx += cw;
            p += chunk_len;
        }

        // Gliding Velocity-Stretched Caret
        if (is_focused) {
            float target_cx = tx + (s_input_cursor > 0 ? font_measure_text(g_ui.font_ui, buffer, s_input_cursor) : 0.0f);
            if (s_input_caret_id != id) {
                s_input_caret = caret_spring_make(target_cx, ty, fh);
                s_input_caret_id = id;
            } else {
                caret_spring_set_target(&s_input_caret, target_cx, ty, fh);
            }
            caret_spring_update(&s_input_caret, g_ui.dt);

            float alpha = caret_spring_get_alpha(&s_input_caret);
            if (alpha > 0.05f) {
                Rect cr = caret_spring_get_bounds(&s_input_caret, 2.0f);
                r_draw_rect_rounded(cr, 1.0f, color_with_alpha(t->primary, alpha));
            }
        }
    } else {
        if (placeholder) {
            font_draw_text(g_ui.font_ui, placeholder, 0, tx, ty, t->input_placeholder);
        }
        if (is_focused) {
            if (s_input_caret_id != id) {
                s_input_caret = caret_spring_make(tx, ty, fh);
                s_input_caret_id = id;
            } else {
                caret_spring_set_target(&s_input_caret, tx, ty, fh);
            }
            caret_spring_update(&s_input_caret, g_ui.dt);

            float alpha = caret_spring_get_alpha(&s_input_caret);
            if (alpha > 0.05f) {
                Rect cr = caret_spring_get_bounds(&s_input_caret, 2.0f);
                r_draw_rect_rounded(cr, 1.0f, color_with_alpha(t->primary, alpha));
            }
        }
    }

    r_pop_clip();
    return changed;
}

bool ui_input_text(const char* id_str, char* buffer, size_t buffer_size, const char* placeholder, Rect rect) {
    return ui_input_text_icon(id_str, ICON_NONE, buffer, buffer_size, placeholder, rect);
}

#define MAX_SCROLL_CONTAINERS 64

typedef struct {
    uint64_t id;
    float current_y;
    float target_y;
    float max_scroll;
    float alpha;
    float thumb_w;
    float fade_timer;
    bool is_dragging;
    float drag_mouse_start_y;
    float drag_scroll_start_y;
    bool in_use;
} ScrollPhysics;

static ScrollPhysics s_scroll_states[MAX_SCROLL_CONTAINERS] = {0};

static ScrollPhysics* get_or_create_scroll_physics(uint64_t id) {
    for (int i = 0; i < MAX_SCROLL_CONTAINERS; i++) {
        if (s_scroll_states[i].in_use && s_scroll_states[i].id == id) {
            return &s_scroll_states[i];
        }
    }
    for (int i = 0; i < MAX_SCROLL_CONTAINERS; i++) {
        if (!s_scroll_states[i].in_use) {
            s_scroll_states[i].in_use = true;
            s_scroll_states[i].id = id;
            s_scroll_states[i].current_y = 0.0f;
            s_scroll_states[i].target_y = 0.0f;
            s_scroll_states[i].max_scroll = 0.0f;
            s_scroll_states[i].alpha = 0.0f;
            s_scroll_states[i].thumb_w = 4.0f;
            s_scroll_states[i].fade_timer = 0.0f;
            s_scroll_states[i].is_dragging = false;
            return &s_scroll_states[i];
        }
    }
    return &s_scroll_states[0];
}

float ui_smooth_scroll(uint64_t id, Rect container_rect, float content_height, float* inout_scroll_y) {
    ScrollPhysics* s = get_or_create_scroll_physics(id);

    float view_h = container_rect.h;
    s->max_scroll = (content_height > view_h) ? (content_height - view_h) : 0.0f;

    // Synchronize if externally initialized with non-zero
    if (inout_scroll_y && s->target_y == 0.0f && *inout_scroll_y > 0.0f && !s->is_dragging) {
        s->current_y = s->target_y = *inout_scroll_y;
    }

    bool container_hovered = rect_contains(container_rect, g_ui.mouse_x, g_ui.mouse_y);

    // Track geometry (on right edge of container)
    float track_w = 12.0f;
    Rect track_rect = rect_make(container_rect.x + container_rect.w - track_w - 2.0f, container_rect.y + 2.0f, track_w, view_h - 4.0f);
    bool track_hovered = rect_contains(track_rect, g_ui.mouse_x, g_ui.mouse_y);

    // Calculate thumb height and position
    float thumb_h = (content_height > 0.0f) ? (view_h / content_height) * (track_rect.h) : track_rect.h;
    if (thumb_h < 32.0f) thumb_h = 32.0f;
    if (thumb_h > track_rect.h) thumb_h = track_rect.h;

    float scroll_frac = (s->max_scroll > 0.0f) ? (s->current_y / s->max_scroll) : 0.0f;
    if (scroll_frac < 0.0f) scroll_frac = 0.0f;
    if (scroll_frac > 1.0f) scroll_frac = 1.0f;

    float thumb_y = track_rect.y + scroll_frac * (track_rect.h - thumb_h);
    float dt = (g_ui.dt > 0.0f && g_ui.dt < 0.1f) ? g_ui.dt : 0.016f;

    float target_thumb_w = (s->is_dragging || track_hovered) ? 8.0f : 6.0f;
    s->thumb_w += (target_thumb_w - s->thumb_w) * (1.0f - expf(-18.0f * dt));

    Rect thumb_rect = rect_make(track_rect.x + (track_rect.w - s->thumb_w) * 0.5f, thumb_y, s->thumb_w, thumb_h);
    bool thumb_hovered = rect_contains(thumb_rect, g_ui.mouse_x, g_ui.mouse_y);

    // Kinetic Mouse Wheel input
    if (container_hovered && g_ui.scroll_y != 0.0f) {
        s->target_y -= g_ui.scroll_y * 72.0f;
        s->alpha = 1.0f;
        s->fade_timer = 2.0f;
    }

    // Scrollbar Dragging & Track Clicking
    if (thumb_hovered && g_ui.mouse_clicked[0]) {
        s->is_dragging = true;
        s->drag_mouse_start_y = g_ui.mouse_y;
        s->drag_scroll_start_y = s->current_y;
        s->alpha = 1.0f;
        s->fade_timer = 2.0f;
    } else if (track_hovered && !thumb_hovered && g_ui.mouse_clicked[0]) {
        if (g_ui.mouse_y < thumb_y) {
            s->target_y -= view_h * 0.85f;
        } else {
            s->target_y += view_h * 0.85f;
        }
        s->alpha = 1.0f;
        s->fade_timer = 2.0f;
    }

    if (s->is_dragging) {
        if (g_ui.mouse_down[0]) {
            float dy = g_ui.mouse_y - s->drag_mouse_start_y;
            float track_avail = track_rect.h - thumb_h;
            float new_scroll = s->drag_scroll_start_y + (track_avail > 0.0f ? (dy / track_avail) * s->max_scroll : 0.0f);
            if (new_scroll < 0.0f) new_scroll = 0.0f;
            if (new_scroll > s->max_scroll) new_scroll = s->max_scroll;
            s->target_y = s->current_y = new_scroll;
            s->alpha = 1.0f;
            s->fade_timer = 2.0f;
        } else {
            s->is_dragging = false;
        }
    }

    // Target Bounds Clamping
    if (s->target_y < 0.0f) s->target_y = 0.0f;
    if (s->target_y > s->max_scroll) s->target_y = s->max_scroll;

    // Physics Simulation (Exponential Smooth Damping)
    if (!s->is_dragging) {
        float speed = 26.0f;
        s->current_y += (s->target_y - s->current_y) * (1.0f - expf(-speed * dt));
        if (fabsf(s->target_y - s->current_y) < 0.1f) {
            s->current_y = s->target_y;
        }
    }

    // Alpha Fade Animation (idle base alpha 0.4f, active 1.0f)
    float base_alpha = 0.4f;
    if (container_hovered || track_hovered || s->is_dragging) {
        s->alpha += (1.0f - s->alpha) * (1.0f - expf(-14.0f * dt));
    } else {
        s->fade_timer -= dt;
        if (s->fade_timer <= 0.0f) {
            s->alpha += (base_alpha - s->alpha) * (1.0f - expf(-6.0f * dt));
        }
    }

    if (inout_scroll_y) {
        *inout_scroll_y = s->current_y;
    }
    return s->current_y;
}

void ui_draw_scrollbar(uint64_t id, Rect container_rect, float content_height) {
    ScrollPhysics* s = get_or_create_scroll_physics(id);
    Theme* t = theme_get_active();

    float view_h = container_rect.h;
    if (content_height <= view_h || s->max_scroll <= 0.0f) {
        return;
    }

    float track_w = 12.0f;
    Rect track_rect = rect_make(container_rect.x + container_rect.w - track_w - 2.0f, container_rect.y + 2.0f, track_w, view_h - 4.0f);
    bool track_hovered = rect_contains(track_rect, g_ui.mouse_x, g_ui.mouse_y);

    float thumb_h = (view_h / content_height) * (track_rect.h);
    if (thumb_h < 32.0f) thumb_h = 32.0f;
    if (thumb_h > track_rect.h) thumb_h = track_rect.h;

    float scroll_frac = (s->max_scroll > 0.0f) ? (s->current_y / s->max_scroll) : 0.0f;
    if (scroll_frac < 0.0f) scroll_frac = 0.0f;
    if (scroll_frac > 1.0f) scroll_frac = 1.0f;

    float thumb_y = track_rect.y + scroll_frac * (track_rect.h - thumb_h);
    Rect thumb_rect = rect_make(track_rect.x + (track_rect.w - s->thumb_w) * 0.5f, thumb_y, s->thumb_w, thumb_h);
    bool thumb_hovered = rect_contains(thumb_rect, g_ui.mouse_x, g_ui.mouse_y);

    if (track_hovered || s->is_dragging) {
        r_draw_rect_rounded(track_rect, 4.0f, color_rgba(1.0f, 1.0f, 1.0f, 0.04f * s->alpha));
    }

    Color thumb_base = s->is_dragging ? t->primary : ((thumb_hovered || track_hovered) ? t->foreground : t->text_muted);
    float thumb_opacity = (s->is_dragging ? 0.95f : ((thumb_hovered || track_hovered) ? 0.8f : 0.5f)) * s->alpha;
    r_draw_rect_rounded(thumb_rect, s->thumb_w * 0.5f, color_with_alpha(thumb_base, thumb_opacity));
}

static void editor_ensure_cap(char** content_ptr, size_t* content_cap, size_t needed) {
    if (needed + 1 >= *content_cap) {
        size_t new_cap = (needed + 512) * 2;
        *content_ptr = realloc(*content_ptr, new_cap);
        *content_cap = new_cap;
    }
}



int ui_tab_bar_icons(const char* id_str, const IconType* icons, const char** tab_labels, int tab_count, int active_index, Rect rect) {
    (void)id_str;
    Theme* t = theme_get_active();
    r_draw_rect(rect, t->tabbar_bg);
    r_draw_line((Vec2){rect.x, rect.y + rect.h}, (Vec2){rect.x + rect.w, rect.y + rect.h}, 1.0f, t->border);

    r_push_clip(rect);

    float cur_x = rect.x + 8.0f;
    int clicked_index = active_index;
    float fh = font_get_height(g_ui.font_ui);

    // Measure total required width
    float total_needed = 16.0f;
    for (int i = 0; i < tab_count; i++) {
        float tw = font_measure_text(g_ui.font_ui, tab_labels[i], 0);
        float icon_space = (icons && icons[i] != ICON_NONE) ? 18.0f : 0.0f;
        total_needed += tw + icon_space + 24.0f;
    }
    bool is_compact = (total_needed > rect.w && rect.w > 0.0f);

    for (int i = 0; i < tab_count; i++) {
        const char* label = tab_labels[i];
        IconType icon = icons ? icons[i] : ICON_NONE;
        float tw = font_measure_text(g_ui.font_ui, label, 0);
        float icon_space = (icon != ICON_NONE) ? 18.0f : 0.0f;
        float tab_w = tw + icon_space + (is_compact ? 12.0f : 20.0f);
        if (cur_x + tab_w > rect.x + rect.w) {
            tab_w = rect.x + rect.w - cur_x;
            if (tab_w < 20.0f) tab_w = 20.0f;
        }

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

        Color text_col = is_active ? t->foreground : (hovered ? t->foreground : t->text_muted);
        float content_x = tr.x + (is_compact ? 6.0f : 10.0f);
        if (icon != ICON_NONE) {
            float isz = 13.0f;
            Rect ir = rect_make(content_x, tr.y + (tr.h - isz) * 0.5f, isz, isz);
            icon_draw(icon, ir, is_active ? t->primary : text_col);
            content_x += 18.0f;
        }

        float ty = tr.y + (tr.h - fh) * 0.5f;
        if (content_x < tr.x + tr.w - 4.0f) {
            font_draw_text(g_ui.font_ui, label, 0, content_x, ty, text_col);
        }

        cur_x += tab_w + (is_compact ? 2.0f : 4.0f);
    }

    r_pop_clip();
    return clicked_index;
}

int ui_tab_bar(const char* id_str, const char** tab_labels, int tab_count, int active_index, Rect rect) {
    return ui_tab_bar_icons(id_str, NULL, tab_labels, tab_count, active_index, rect);
}


bool ui_dropdown(const char* id_str, const char** items, int item_count, int* selected_index, Rect rect) {
    Theme* t = theme_get_active();
    uint64_t id = ui_hash_id(id_str, 0);
    bool is_open = (s_dropdown_overlay.id == id && s_dropdown_overlay.is_open);
    bool hovered = rect_contains(rect, g_ui.mouse_x, g_ui.mouse_y);

    if (hovered && g_ui.mouse_clicked[0]) {
        if (is_open) {
            s_dropdown_overlay.is_open = false;
        } else {
            s_dropdown_overlay.id = id;
            s_dropdown_overlay.is_open = true;
            s_dropdown_overlay.just_opened = true;
            s_dropdown_overlay.item_count = item_count > 32 ? 32 : item_count;
            for (int i = 0; i < s_dropdown_overlay.item_count; i++) {
                if (items && items[i]) {
                    strncpy(s_dropdown_overlay.items[i], items[i], sizeof(s_dropdown_overlay.items[i]) - 1);
                    s_dropdown_overlay.items[i][sizeof(s_dropdown_overlay.items[i]) - 1] = '\0';
                } else {
                    s_dropdown_overlay.items[i][0] = '\0';
                }
            }
            s_dropdown_overlay.selected_index = selected_index;
            s_dropdown_overlay.anchor_rect = rect;
            s_dropdown_overlay.changed = false;
        }
        is_open = s_dropdown_overlay.is_open;
    }

    Color bg = is_open ? t->secondary_active : (hovered ? t->secondary_hover : t->input_bg);
    Color bdr = is_open ? t->primary : t->input_border;
    r_draw_rect_rounded_bordered(rect, 4.0f, 1.0f, bdr, bg);

    int cur_sel = (selected_index && *selected_index >= 0 && *selected_index < item_count) ? *selected_index : 0;
    const char* current_label = (item_count > 0 && items && items[cur_sel]) ? items[cur_sel] : "";

    float fh_bold = font_get_height(g_ui.font_bold);
    float tx = rect.x + 8.0f;
    float ty = rect.y + (rect.h - fh_bold) * 0.5f;
    Color txt_col = theme_get_method_color(current_label);
    font_draw_text(g_ui.font_bold, current_label, 0, tx, ty, txt_col);

    // Chevron Icon
    Rect ch_rect = rect_make(rect.x + rect.w - 18.0f, rect.y + (rect.h - 12.0f) * 0.5f, 12.0f, 12.0f);
    icon_draw(is_open ? ICON_CHEVRON_DOWN : ICON_CHEVRON_RIGHT, ch_rect, t->text_muted);

    bool res = false;
    if (s_dropdown_overlay.id == id && s_dropdown_overlay.changed) {
        res = true;
        s_dropdown_overlay.changed = false;
    }
    return res;
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


bool ui_data_table(const char* id_str, const char** columns, size_t col_count,
                   char*** rows, size_t row_count, const char* filter_text,
                   int* selected_row, int* selected_col, Rect rect, float* scroll_y) {
    (void)selected_col;
    Theme* t = theme_get_active();
    if (!columns || col_count == 0 || !rows) return false;

    float row_h = 24.0f;
    float header_h = 26.0f;
    uint64_t id = ui_hash_id(id_str ? id_str : "data_table", 0);

    float total_content_h = header_h + row_count * row_h + 10.0f;
    float current_scroll = ui_smooth_scroll(id, rect, total_content_h, scroll_y);

    r_draw_rect(rect, t->background);

    float col_w = (rect.w - 10.0f) / (float)col_count;
    if (col_w < 110.0f) col_w = 110.0f;

    // Header
    float h_fh = font_get_height(g_ui.font_bold);
    Rect h_rect = rect_make(rect.x, rect.y, rect.w, header_h);
    r_draw_rect(h_rect, t->panel_bg);
    r_draw_line((Vec2){rect.x, rect.y + header_h}, (Vec2){rect.x + rect.w, rect.y + header_h}, 1.0f, t->border);

    for (size_t c = 0; c < col_count; c++) {
        Rect cell = rect_make(rect.x + c * col_w, rect.y, col_w, header_h);
        r_draw_line((Vec2){cell.x + cell.w, cell.y}, (Vec2){cell.x + cell.w, cell.y + cell.h}, 1.0f, t->border_subtle);
        font_draw_text(g_ui.font_bold, columns[c], 0, cell.x + 8.0f, cell.y + (header_h - h_fh) * 0.5f, t->foreground);
    }

    // Rows
    float r_fh = font_get_height(g_ui.font_ui);
    r_push_clip(rect_make(rect.x, rect.y + header_h, rect.w, rect.h - header_h));
    float cur_y = rect.y + header_h - current_scroll;

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
            bool on_scrollbar = (total_content_h > rect.h) && (g_ui.mouse_x >= rect.x + rect.w - 18.0f);
            bool row_hovered = !on_scrollbar && rect_contains(row_rect, g_ui.mouse_x, g_ui.mouse_y);
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
                font_draw_text(g_ui.font_ui, rows[r][c], 0, cell.x + 8.0f, cell.y + (row_h - r_fh) * 0.5f, is_sel ? t->text_selected : t->foreground);
                r_pop_clip();
            }
        }

        cur_y += row_h;
    }

    r_pop_clip();
    ui_draw_scrollbar(id, rect, total_content_h);
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

    float m_fh = font_get_height(g_ui.font_bold);
    r_draw_line((Vec2){mx, my + 40.0f}, (Vec2){mx + w, my + 40.0f}, 1.0f, t->border);
    font_draw_text(g_ui.font_bold, title, 0, mx + 16.0f, my + (40.0f - m_fh) * 0.5f, t->foreground);

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
        icon_draw(icon, rect_make(cur_x, r.y + (item_h - 12.0f) * 0.5f, 12.0f, 12.0f), hovered ? t->foreground : t->text_muted);
        cur_x += 16.0f;
    }

    float cm_fh = font_get_height(g_ui.font_ui);
    font_draw_text(g_ui.font_ui, label, 0, cur_x, r.y + (item_h - cm_fh) * 0.5f, hovered ? t->foreground : t->text_muted);
    *y_cursor += item_h + 2.0f;

    return clicked;
}

void ui_context_menu_end(void) {
    s_cm_active_id = 0;
}

void ui_code_editor_open_search_legacy(const char* id_str) {
    if (!id_str) return;
    uint64_t id = ui_hash_id(id_str, 0);
    for (int i = 0; i < MAX_EDITOR_SLOTS; i++) {
        if (s_editor_slots[i].id == id && s_editor_slots[i].buf) {
            s_editor_slots[i].search.is_open = true;
            s_editor_slots[i].search.matches_dirty = true;
            char s_in_id[64];
            snprintf(s_in_id, sizeof(s_in_id), "srch_%lu", (unsigned long)id);
            g_ui.focus_id = ui_hash_id(s_in_id, 0);
            return;
        }
    }
}


bool ui_code_editor_legacy(const char* id_str, char** content_ptr, size_t* content_len, size_t* content_cap,
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

    // Connect to high-performance cached FredBuffer slot
    EditorBufferSlot* slot = get_or_create_editor_slot(id, *content_ptr, *content_ptr);
    FredBuffer* buf = slot->buf;
    char* text = slot->cached_text;
    size_t len = slot->cached_text_len;
    if (content_len) *content_len = len;

    float gutter_w = show_line_numbers ? 42.0f : 8.0f;
    float line_h = 20.0f;
    float text_x = rect.x + gutter_w + 10.0f;
    float wrap_w = rect.w - gutter_w - 24.0f;
    if (wrap_w < 100.0f) wrap_w = 100.0f;

    // Multi-line Soft Word-Wrapping Layout with caching
    if (!slot->layout_valid || fabsf(slot->last_wrap_w - wrap_w) > 1.0f) {
        if (slot->layout_valid) {
            font_layout_free(&slot->layout);
        }
        slot->layout = font_layout_text_wrapped(g_ui.font_mono, text, len, wrap_w, line_h, NULL);
        slot->last_wrap_w = wrap_w;
        slot->layout_valid = true;
    }
    TextLayout* layout = &slot->layout;

    float total_content_h = layout->total_height + 20.0f;
    float current_scroll = ui_smooth_scroll(id, rect, total_content_h, scroll_y);
    float start_y = rect.y + 6.0f - current_scroll;

    ScrollPhysics* sp = get_or_create_scroll_physics(id);
    bool on_scrollbar = (total_content_h > rect.h) && (g_ui.mouse_x >= rect.x + rect.w - 18.0f);
    bool on_search_bar = false;
    if (slot->search.is_open) {
        float bar_w = 340.0f;
        if (bar_w > rect.w - 20.0f) bar_w = rect.w - 20.0f;
        float bar_x = rect.x + rect.w - bar_w - 20.0f;
        float bar_y = rect.y + 6.0f;
        Rect bar_rect = rect_make(bar_x, bar_y, bar_w, 32.0f);
        on_search_bar = rect_contains(bar_rect, g_ui.mouse_x, g_ui.mouse_y);
    }

    // If scrolling with mouse wheel, stop any drag selection immediately
    if (g_ui.scroll_y != 0.0f || g_ui.scroll_x != 0.0f || !g_ui.mouse_down[0] || sp->is_dragging) {
        g_ui.is_selecting = false;
    }

    bool cursor_moved_by_input = false;

    // Mouse selection logic with Soft-Wrap Hit-Testing
    if (hovered && !on_scrollbar && !on_search_bar && !sp->is_dragging && g_ui.mouse_clicked[0]) {
        g_ui.focus_id = id;
        is_focused = true;
        g_ui.editor_sel_id = id;
        size_t off = font_layout_get_byte_offset_at_point(layout, g_ui.font_mono, text, g_ui.mouse_x, g_ui.mouse_y, text_x, start_y);

        if (g_ui.mouse_click_count == 2) {
            find_word_bounds(text, len, off, &g_ui.sel_start, &g_ui.sel_end);
            g_ui.is_selecting = false;
        } else if (g_ui.mouse_click_count == 3) {
            size_t l_start = off, l_end = off;
            while (l_start > 0 && text[l_start - 1] != '\n') l_start--;
            while (l_end < len && text[l_end] != '\n') l_end++;
            g_ui.sel_start = l_start;
            g_ui.sel_end = (l_end < len) ? l_end + 1 : l_end;
            g_ui.is_selecting = false;
        } else {
            g_ui.sel_start = g_ui.sel_end = off;
            g_ui.is_selecting = true;
        }
    }

    if (g_ui.is_selecting && !sp->is_dragging && g_ui.editor_sel_id == id && g_ui.mouse_down[0] && g_ui.scroll_y == 0.0f) {
        g_ui.sel_end = font_layout_get_byte_offset_at_point(layout, g_ui.font_mono, text, g_ui.mouse_x, g_ui.mouse_y, text_x, start_y);
        cursor_moved_by_input = true;
    }

    if (g_ui.mouse_released[0] || !g_ui.mouse_down[0] || sp->is_dragging) {
        g_ui.is_selecting = false;
    }

    // Keyboard navigation & mutation handling
    bool changed = false;
    if (is_focused) {
        if (g_ui.sel_start > len) g_ui.sel_start = len;
        if (g_ui.sel_end > len) g_ui.sel_end = len;

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
            cursor_moved_by_input = true;
        } else if (g_ui.key_pressed == GLFW_KEY_RIGHT) {
            if (g_ui.ctrl_down) {
                g_ui.sel_end = find_word_next(text, len, g_ui.sel_end);
            } else if (g_ui.sel_end < len) {
                g_ui.sel_end++;
            }
            if (!g_ui.shift_down) g_ui.sel_start = g_ui.sel_end;
            cursor_moved_by_input = true;
        } else if (g_ui.key_pressed == GLFW_KEY_UP) {
            float rel_cx = 0.0f, rel_cy = 0.0f;
            font_layout_get_cursor_pos(layout, g_ui.font_mono, text, g_ui.sel_end, &rel_cx, &rel_cy, &line_h);
            g_ui.sel_end = font_layout_get_byte_offset_at_point(layout, g_ui.font_mono, text, text_x + rel_cx, start_y + rel_cy - line_h * 0.5f, text_x, start_y);
            if (!g_ui.shift_down) g_ui.sel_start = g_ui.sel_end;
            cursor_moved_by_input = true;
        } else if (g_ui.key_pressed == GLFW_KEY_DOWN) {
            float rel_cx = 0.0f, rel_cy = 0.0f;
            font_layout_get_cursor_pos(layout, g_ui.font_mono, text, g_ui.sel_end, &rel_cx, &rel_cy, &line_h);
            g_ui.sel_end = font_layout_get_byte_offset_at_point(layout, g_ui.font_mono, text, text_x + rel_cx, start_y + rel_cy + line_h * 1.5f, text_x, start_y);
            if (!g_ui.shift_down) g_ui.sel_start = g_ui.sel_end;
            cursor_moved_by_input = true;
        } else if (g_ui.key_pressed == GLFW_KEY_PAGE_UP) {
            sp->target_y -= rect.h * 0.85f;
            sp->alpha = 1.0f;
        } else if (g_ui.key_pressed == GLFW_KEY_PAGE_DOWN) {
            sp->target_y += rect.h * 0.85f;
            sp->alpha = 1.0f;
        } else if (g_ui.key_pressed == GLFW_KEY_HOME) {
            if (g_ui.ctrl_down) {
                g_ui.sel_end = 0;
            } else {
                size_t v_line = 0;
                for (size_t li = 0; li < layout->line_count; li++) {
                    if (li == layout->line_count - 1 || g_ui.sel_end < layout->lines[li + 1].start_byte) {
                        v_line = li;
                        break;
                    }
                }
                size_t v_start = layout->lines[v_line].start_byte;
                if (g_ui.sel_end > v_start) {
                    g_ui.sel_end = v_start;
                } else {
                    while (g_ui.sel_end > 0 && text[g_ui.sel_end - 1] != '\n') g_ui.sel_end--;
                }
            }
            if (!g_ui.shift_down) g_ui.sel_start = g_ui.sel_end;
            cursor_moved_by_input = true;
        } else if (g_ui.key_pressed == GLFW_KEY_END) {
            if (g_ui.ctrl_down) {
                g_ui.sel_end = len;
            } else {
                size_t v_line = 0;
                for (size_t li = 0; li < layout->line_count; li++) {
                    if (li == layout->line_count - 1 || g_ui.sel_end < layout->lines[li + 1].start_byte) {
                        v_line = li;
                        break;
                    }
                }
                size_t v_end = layout->lines[v_line].start_byte + layout->lines[v_line].length_byte;
                if (g_ui.sel_end < v_end) {
                    g_ui.sel_end = v_end;
                } else {
                    while (g_ui.sel_end < len && text[g_ui.sel_end] != '\n') g_ui.sel_end++;
                }
            }
            if (!g_ui.shift_down) g_ui.sel_start = g_ui.sel_end;
            cursor_moved_by_input = true;
        }

        // Selection & Clipboard shortcuts
        if (g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_A) {
            g_ui.sel_start = 0;
            g_ui.sel_end = len;
        } else if (g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_C) {
            if (has_sel) {
                size_t slen = s_max - s_min;
                char* clip = malloc(slen + 1);
                memcpy(clip, text + s_min, slen);
                clip[slen] = '\0';
                glfwSetClipboardString(NULL, clip);
                free(clip);
            } else {
                glfwSetClipboardString(NULL, text);
            }
        } else if (g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_F) {
            slot->search.is_open = true;
            if (has_sel && (s_max - s_min) < sizeof(slot->search.query)) {
                size_t slen = s_max - s_min;
                memcpy(slot->search.query, text + s_min, slen);
                slot->search.query[slen] = '\0';
            }
            slot->search.matches_dirty = true;
            char s_in_id[64];
            snprintf(s_in_id, sizeof(s_in_id), "srch_%lu", (unsigned long)id);
            g_ui.focus_id = ui_hash_id(s_in_id, 0);
            g_ui.sel_start = 0;
            g_ui.sel_end = strlen(slot->search.query);
        }

        // Undo (Ctrl+Z) / Redo (Ctrl+Y)
        if (!is_readonly && g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_Z) {
            fredbuf_undo(buf, &g_ui.sel_end);
            g_ui.sel_start = g_ui.sel_end;
            changed = true;
            cursor_moved_by_input = true;
        } else if (!is_readonly && g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_Y) {
            fredbuf_redo(buf, &g_ui.sel_end);
            g_ui.sel_start = g_ui.sel_end;
            changed = true;
            cursor_moved_by_input = true;
        }

        // Mutation keys
        if (!is_readonly && content_cap) {
            // Cut (Ctrl+X)
            if (g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_X && has_sel) {
                size_t slen = s_max - s_min;
                char* clip_buf = malloc(slen + 1);
                memcpy(clip_buf, text + s_min, slen);
                clip_buf[slen] = '\0';
                glfwSetClipboardString(NULL, clip_buf);
                free(clip_buf);

                fredbuf_delete_range(buf, s_min, slen);
                g_ui.sel_start = g_ui.sel_end = s_min;
                changed = true;
                cursor_moved_by_input = true;
            }
            // Paste (Ctrl+V)
            else if (g_ui.ctrl_down && g_ui.key_pressed == GLFW_KEY_V) {
                const char* clip = glfwGetClipboardString(NULL);
                if (clip && *clip) {
                    if (has_sel) {
                        fredbuf_delete_range(buf, s_min, s_max - s_min);
                        g_ui.sel_end = s_min;
                    }
                    size_t clen = strlen(clip);
                    fredbuf_insert_string(buf, g_ui.sel_end, clip, clen);
                    g_ui.sel_start = g_ui.sel_end = g_ui.sel_end + clen;
                    changed = true;
                    cursor_moved_by_input = true;
                }
            }
            // Backspace
            else if (g_ui.key_pressed == GLFW_KEY_BACKSPACE) {
                if (has_sel) {
                    fredbuf_delete_range(buf, s_min, s_max - s_min);
                    g_ui.sel_start = g_ui.sel_end = s_min;
                    changed = true;
                } else if (g_ui.sel_end > 0) {
                    fredbuf_delete_range(buf, g_ui.sel_end - 1, 1);
                    g_ui.sel_end--;
                    g_ui.sel_start = g_ui.sel_end;
                    changed = true;
                }
                cursor_moved_by_input = true;
            }
            // Delete
            else if (g_ui.key_pressed == GLFW_KEY_DELETE) {
                if (has_sel) {
                    fredbuf_delete_range(buf, s_min, s_max - s_min);
                    g_ui.sel_start = g_ui.sel_end = s_min;
                    changed = true;
                } else if (g_ui.sel_end < len) {
                    fredbuf_delete_range(buf, g_ui.sel_end, 1);
                    changed = true;
                }
                cursor_moved_by_input = true;
            }
            // Enter key (newline + auto indent)
            else if (g_ui.key_pressed == GLFW_KEY_ENTER) {
                if (has_sel) {
                    fredbuf_delete_range(buf, s_min, s_max - s_min);
                    g_ui.sel_end = s_min;
                }
                size_t lstart = g_ui.sel_end;
                while (lstart > 0 && text[lstart - 1] != '\n') lstart--;
                size_t indent_len = 0;
                while (lstart + indent_len < len && (text[lstart + indent_len] == ' ' || text[lstart + indent_len] == '\t')) {
                    indent_len++;
                }
                char nl_buf[64] = "\n";
                if (indent_len > 0 && indent_len < 60) {
                    memcpy(nl_buf + 1, text + lstart, indent_len);
                    nl_buf[1 + indent_len] = '\0';
                }
                fredbuf_insert_string(buf, g_ui.sel_end, nl_buf, 1 + indent_len);
                g_ui.sel_start = g_ui.sel_end = g_ui.sel_end + 1 + indent_len;
                changed = true;
                cursor_moved_by_input = true;
            }
            // Tab key (2 spaces)
            else if (g_ui.key_pressed == GLFW_KEY_TAB) {
                if (has_sel) {
                    fredbuf_delete_range(buf, s_min, s_max - s_min);
                    g_ui.sel_end = s_min;
                }
                fredbuf_insert_string(buf, g_ui.sel_end, "  ", 2);
                g_ui.sel_start = g_ui.sel_end = g_ui.sel_end + 2;
                changed = true;
                cursor_moved_by_input = true;
            }
            // Character typing
            else if (g_ui.char_input_len > 0) {
                if (has_sel) {
                    fredbuf_delete_range(buf, s_min, s_max - s_min);
                    g_ui.sel_end = s_min;
                }
                size_t in_len = 0;
                char temp_in[32];
                for (size_t i = 0; i < g_ui.char_input_len; i++) {
                    if (g_ui.char_input[i] >= 32) {
                        temp_in[in_len++] = g_ui.char_input[i];
                    }
                }
                if (in_len > 0) {
                    fredbuf_insert_string(buf, g_ui.sel_end, temp_in, in_len);
                    g_ui.sel_start = g_ui.sel_end = g_ui.sel_end + in_len;
                    changed = true;
                    cursor_moved_by_input = true;
                }
            }
        }
    }

    // Synchronize FredBuffer to content_ptr when modified
    if (changed) {
        if (slot->cached_text) free(slot->cached_text);
        slot->cached_text = fredbuf_to_string(buf);
        slot->cached_text_len = fredbuf_length(buf);
        text = slot->cached_text;
        len = slot->cached_text_len;
        if (content_len) *content_len = len;

        editor_ensure_cap(content_ptr, content_cap, len);
        memcpy(*content_ptr, text, len);
        (*content_ptr)[len] = '\0';
        slot->last_source_ptr = *content_ptr;
        slot->last_source_len = len;

        // Invalidate layout & tokens
        if (slot->layout_valid) {
            font_layout_free(&slot->layout);
            slot->layout_valid = false;
        }
        slot->tokens_valid = false;

        slot->layout = font_layout_text_wrapped(g_ui.font_mono, text, len, wrap_w, line_h, NULL);
        slot->last_wrap_w = wrap_w;
        slot->layout_valid = true;
        layout = &slot->layout;
        total_content_h = layout->total_height + 20.0f;
    }

    // Fast Boyer-Moore-Horspool Search Index Update
    if (slot->search.matches_dirty) {
        if (slot->search.matches) {
            free(slot->search.matches);
            slot->search.matches = NULL;
        }
        slot->search.match_count = 0;
        size_t qlen = strlen(slot->search.query);
        if (qlen > 0) {
            fredbuf_search_text(text, len, slot->search.query, qlen, slot->search.case_sensitive, &slot->search.matches, &slot->search.match_count);
            if (slot->search.match_count > 0) {
                slot->search.active_match_idx = 0;
                for (size_t mi = 0; mi < slot->search.match_count; mi++) {
                    if (slot->search.matches[mi].start >= g_ui.sel_end) {
                        slot->search.active_match_idx = (int)mi;
                        break;
                    }
                }
            } else {
                slot->search.active_match_idx = -1;
            }
        } else {
            slot->search.active_match_idx = -1;
        }
        slot->search.matches_dirty = false;
    }

    // High performance syntax token cache update
    if (is_json_syntax && !slot->tokens_valid) {
        if (slot->cached_tokens) free(slot->cached_tokens);
        slot->cached_tokens = json_tokenize(text, len, &slot->token_count);
        slot->tokens_valid = true;
    }
    JsonToken* tokens = is_json_syntax ? slot->cached_tokens : NULL;
    size_t token_count = is_json_syntax ? slot->token_count : 0;

    r_draw_rect(rect, t->background);

    if (show_line_numbers) {
        r_draw_rect(rect_make(rect.x, rect.y, gutter_w, rect.h), t->panel_bg);
        r_draw_line((Vec2){rect.x + gutter_w, rect.y}, (Vec2){rect.x + gutter_w, rect.y + rect.h}, 1.0f, t->border);
    }

    r_push_clip(rect_make(rect.x, rect.y, rect.w, rect.h));

    size_t s_min = (g_ui.editor_sel_id == id && g_ui.sel_start != g_ui.sel_end) ? (g_ui.sel_start < g_ui.sel_end ? g_ui.sel_start : g_ui.sel_end) : 0;
    size_t s_max = (g_ui.editor_sel_id == id && g_ui.sel_start != g_ui.sel_end) ? (g_ui.sel_start > g_ui.sel_end ? g_ui.sel_start : g_ui.sel_end) : 0;
    bool has_sel = (s_min != s_max);

    float cursor_draw_x = -1.0f;
    float cursor_draw_y = -1.0f;

    // Accurate cursor position mapping on wrapped line geometry
    if (is_focused) {
        float rel_cx = 0.0f, rel_cy = 0.0f;
        font_layout_get_cursor_pos(layout, g_ui.font_mono, text, g_ui.sel_end, &rel_cx, &rel_cy, &line_h);
        cursor_draw_x = text_x + rel_cx;
        cursor_draw_y = start_y + rel_cy;

        // Auto-scroll viewport to follow caret smoothly ONLY when user actively moved cursor via keyboard or explicit drag-select
        if (cursor_moved_by_input) {
            if (cursor_draw_y < rect.y + 6.0f) {
                sp->target_y += (cursor_draw_y - (rect.y + 6.0f));
                sp->alpha = 1.0f;
                sp->fade_timer = 1.5f;
            } else if (cursor_draw_y + line_h > rect.y + rect.h - 6.0f) {
                sp->target_y += (cursor_draw_y + line_h - (rect.y + rect.h - 6.0f));
                sp->alpha = 1.0f;
                sp->fade_timer = 1.5f;
            }
        }
    }

    float fh_mono = font_get_height(g_ui.font_mono);

    size_t first_li = 0;
    if (current_scroll > 0.0f && line_h > 0.0f) {
        first_li = (size_t)(current_scroll / line_h);
        if (first_li > 2) first_li -= 2;
        else first_li = 0;
        if (first_li >= layout->line_count) first_li = (layout->line_count > 0) ? layout->line_count - 1 : 0;
    }

    size_t last_li = layout->line_count;
    if (line_h > 0.0f) {
        size_t visible_lines_count = (size_t)(rect.h / line_h) + 4;
        last_li = first_li + visible_lines_count;
        if (last_li > layout->line_count) last_li = layout->line_count;
    }

    // Determine starting logical line in O(log N) using FredBuffer line indexing
    size_t start_logical = 0;
    if (layout->line_count > 0 && first_li < layout->line_count) {
        fredbuf_offset_to_line_col(buf, layout->lines[first_li].start_byte, &start_logical, NULL);
    }
    int logical_line = (int)start_logical + 1;

    // Fast O(log N) binary search for starting token index
    size_t cur_token_idx = 0;
    if (tokens && token_count > 0 && first_li < layout->line_count) {
        size_t target_byte = layout->lines[first_li].start_byte;
        size_t t_low = 0, t_high = token_count - 1;
        while (t_low <= t_high) {
            size_t t_mid = (t_low + t_high) / 2;
            if (tokens[t_mid].start + tokens[t_mid].length <= target_byte) {
                t_low = t_mid + 1;
            } else {
                cur_token_idx = t_mid;
                if (t_mid == 0) break;
                t_high = t_mid - 1;
            }
        }
    }

    for (size_t li = first_li; li < last_li; li++) {
        TextLine* vline = &layout->lines[li];
        float cur_y = start_y + vline->y_offset;

        bool is_continuation = (vline->start_byte > 0 && text[vline->start_byte - 1] != '\n');

        if (cur_y + line_h >= rect.y && cur_y <= rect.y + rect.h) {
            float text_y = cur_y + (line_h - fh_mono) * 0.5f;

            // Line numbers in gutter (only on first slice of logical line)
            if (show_line_numbers) {
                if (!is_continuation) {
                    char lbuf[16];
                    snprintf(lbuf, sizeof(lbuf), "%d", logical_line);
                    float lw = font_measure_text(g_ui.font_mono, lbuf, 0);
                    font_draw_text(g_ui.font_mono, lbuf, 0, rect.x + gutter_w - lw - 8.0f, text_y, t->text_dim);
                } else {
                    // Dim wrapped continuation indicator
                    font_draw_text(g_ui.font_mono, "\xc2\xbb", 0, rect.x + gutter_w - 14.0f, text_y, color_with_alpha(t->text_dim, 0.4f));
                }
            }

            // Selection highlight rect for this visual line slice
            size_t vline_end = vline->start_byte + vline->length_byte;
            if (has_sel && vline->start_byte < s_max && (vline_end > s_min || (vline->length_byte == 0 && vline->start_byte >= s_min))) {
                size_t sel_0 = (vline->start_byte > s_min) ? vline->start_byte : s_min;
                size_t sel_1 = (vline_end < s_max) ? vline_end : s_max;
                if (sel_0 > vline_end) sel_0 = vline_end;
                if (sel_1 > vline_end) sel_1 = vline_end;

                size_t sub_0 = sel_0 - vline->start_byte;
                size_t sub_1 = sel_1 - vline->start_byte;
                float x0 = text_x + (sub_0 > 0 ? font_measure_text(g_ui.font_mono, text + vline->start_byte, sub_0) : 0.0f);
                float x1 = text_x + (sub_1 > 0 ? font_measure_text(g_ui.font_mono, text + vline->start_byte, sub_1) : 0.0f);

                // If the selection extends past the end of this line (e.g. newline selected), add trailing width
                if (vline_end < s_max && (li + 1 < layout->line_count || (vline_end < len && text[vline_end] == '\n'))) {
                    x1 += 8.0f;
                }
                if (x1 > x0) {
                    r_draw_rect_rounded(rect_make(x0, cur_y, x1 - x0, line_h), 2.0f, t->selection_bg);
                }
            }

            // Search match highlights on this visual line slice
            if (slot->search.is_open && slot->search.match_count > 0 && slot->search.matches) {
                for (size_t m = 0; m < slot->search.match_count; m++) {
                    TextRange* match = &slot->search.matches[m];
                    if (match->start + match->length > vline->start_byte && match->start < vline_end) {
                        size_t m_0 = (match->start > vline->start_byte) ? match->start : vline->start_byte;
                        size_t m_1 = (match->start + match->length < vline_end) ? (match->start + match->length) : vline_end;
                        if (m_1 > m_0) {
                            size_t sub_0 = m_0 - vline->start_byte;
                            size_t sub_1 = m_1 - vline->start_byte;
                            float mx0 = text_x + (sub_0 > 0 ? font_measure_text(g_ui.font_mono, text + vline->start_byte, sub_0) : 0.0f);
                            float mx1 = text_x + (sub_1 > 0 ? font_measure_text(g_ui.font_mono, text + vline->start_byte, sub_1) : 0.0f);
                            bool is_cur = ((int)m == slot->search.active_match_idx);
                            Color bg_col = is_cur ? color_rgba(245, 158, 11, 0.85f) : color_rgba(234, 179, 8, 0.35f);
                            Color bdr_col = is_cur ? color_rgba(255, 255, 255, 0.90f) : color_rgba(234, 179, 8, 0.70f);
                            r_draw_rect_rounded_bordered(rect_make(mx0, cur_y + 1.0f, mx1 - mx0, line_h - 2.0f), 2.0f, 1.0f, bdr_col, bg_col);
                        }
                    }
                }
            }

            // Fast O(N) Syntax Highlighting & Token rendering
            if (tokens && token_count > 0) {
                float cur_x = text_x;
                size_t k = vline->start_byte;

                // Sync token index pointer
                while (cur_token_idx < token_count && (tokens[cur_token_idx].start + tokens[cur_token_idx].length) <= k) {
                    cur_token_idx++;
                }

                while (k < vline_end) {
                    if (cur_token_idx < token_count && tokens[cur_token_idx].start <= k && (tokens[cur_token_idx].start + tokens[cur_token_idx].length) > k) {
                        JsonToken* tok = &tokens[cur_token_idx];
                        size_t tok_rem = (tok->start + tok->length) - k;
                        if (k + tok_rem > vline_end) tok_rem = vline_end - k;

                        Color tok_col = t->foreground;
                        switch (tok->type) {
                            case JSON_TOK_KEY: tok_col = t->syntax_key; break;
                            case JSON_TOK_STRING: tok_col = t->syntax_string; break;
                            case JSON_TOK_NUMBER: tok_col = t->syntax_number; break;
                            case JSON_TOK_BOOLEAN: tok_col = t->syntax_boolean; break;
                            case JSON_TOK_NULL: tok_col = t->syntax_null; break;
                            case JSON_TOK_PUNCTUATION: tok_col = t->syntax_punctuation; break;
                            case JSON_TOK_COMMENT: tok_col = t->syntax_comment; break;
                            default: break;
                        }

                        if (tok_rem > 0) {
                            font_draw_text(g_ui.font_mono, text + k, tok_rem, cur_x, text_y, tok_col);
                            cur_x += font_measure_text(g_ui.font_mono, text + k, tok_rem);
                            k += tok_rem;
                        }
                        if (k >= tok->start + tok->length) cur_token_idx++;
                    } else {
                        size_t next_k = vline_end;
                        if (cur_token_idx < token_count && tokens[cur_token_idx].start < next_k && tokens[cur_token_idx].start > k) {
                            next_k = tokens[cur_token_idx].start;
                        }
                        size_t plain_len = next_k - k;
                        if (plain_len > 0) {
                            font_draw_text(g_ui.font_mono, text + k, plain_len, cur_x, text_y, t->foreground);
                            cur_x += font_measure_text(g_ui.font_mono, text + k, plain_len);
                            k += plain_len;
                        }
                    }
                }
            } else {
                if (vline->length_byte > 0) {
                    font_draw_text(g_ui.font_mono, text + vline->start_byte, vline->length_byte, text_x, text_y, t->foreground);
                }
            }
        }

        if (li + 1 < layout->line_count && vline->start_byte + vline->length_byte < len && text[vline->start_byte + vline->length_byte] == '\n') {
            logical_line++;
        }
    }

    // Gliding Velocity-Stretched Harmonic Spring Caret
    static CaretSpring s_editor_caret = {0};
    static uint64_t s_editor_caret_id = 0;

    if (is_focused && cursor_draw_x >= 0.0f) {
        if (s_editor_caret_id != id) {
            s_editor_caret = caret_spring_make(cursor_draw_x, cursor_draw_y, line_h);
            s_editor_caret_id = id;
        } else {
            caret_spring_set_target(&s_editor_caret, cursor_draw_x, cursor_draw_y, line_h);
        }
        caret_spring_update(&s_editor_caret, g_ui.dt);

        float alpha = caret_spring_get_alpha(&s_editor_caret);
        if (alpha > 0.05f) {
            Rect cr = caret_spring_get_bounds(&s_editor_caret, 2.0f);
            r_draw_rect_rounded(cr, 1.0f, color_with_alpha(t->primary, alpha));
        }
    }

    // Top-Right Floating Copy Button (Only in read-only viewer mode when search bar is closed)
    if (is_readonly && text && text[0] && !slot->search.is_open) {
        Rect copy_btn = rect_make(rect.x + rect.w - 70.0f, rect.y + 6.0f, 62.0f, 22.0f);
        if (ui_button_icon(ICON_COPY, "Copy", copy_btn, false)) {
            glfwSetClipboardString(NULL, text);
        }
    }

    // Render Floating Modern Search Bar
    if (slot->search.is_open) {
        float bar_w = 340.0f;
        if (bar_w > rect.w - 24.0f) bar_w = rect.w - 24.0f;
        float bar_h = 32.0f;
        float bar_x = rect.x + rect.w - bar_w - 20.0f;
        float bar_y = rect.y + 6.0f;
        Rect bar_rect = rect_make(bar_x, bar_y, bar_w, bar_h);

        // Background & shadow
        r_draw_rect_rounded(rect_make(bar_x + 1.0f, bar_y + 3.0f, bar_w, bar_h), 5.0f, color_rgba(0, 0, 0, 0.45f));
        r_draw_rect_rounded_bordered(bar_rect, 5.0f, 1.0f, t->border, t->panel_bg);

        // Search Input ID
        char search_in_id[64];
        snprintf(search_in_id, sizeof(search_in_id), "srch_%lu", (unsigned long)id);

        float input_w = 140.0f;
        Rect input_rect = rect_make(bar_x + 4.0f, bar_y + 3.0f, input_w, bar_h - 6.0f);

        // Search query text input
        if (ui_input_text_icon(search_in_id, ICON_SEARCH, slot->search.query, sizeof(slot->search.query), "Find...", input_rect)) {
            slot->search.matches_dirty = true;
        }

        bool jump_next = false;
        bool jump_prev = false;

        uint64_t s_in_hash = ui_hash_id(search_in_id, 0);
        bool search_focused = (g_ui.focus_id == s_in_hash);

        if (search_focused) {
            if (g_ui.key_pressed == GLFW_KEY_ESCAPE) {
                slot->search.is_open = false;
                g_ui.focus_id = id;
            } else if (g_ui.key_pressed == GLFW_KEY_ENTER) {
                if (g_ui.shift_down) jump_prev = true;
                else jump_next = true;
            }
        }

        if (is_focused || search_focused) {
            if (g_ui.key_pressed == GLFW_KEY_F3) {
                if (g_ui.shift_down) jump_prev = true;
                else jump_next = true;
            }
        }

        // Match count label
        float count_x = input_rect.x + input_w + 6.0f;
        char count_buf[64];
        if (slot->search.query[0] == '\0') {
            count_buf[0] = '\0';
        } else if (slot->search.match_count == 0) {
            snprintf(count_buf, sizeof(count_buf), "0/0");
        } else {
            snprintf(count_buf, sizeof(count_buf), "%d/%zu", slot->search.active_match_idx + 1, slot->search.match_count);
        }

        float count_fh = font_get_height(g_ui.font_ui);
        Color count_col = (slot->search.match_count > 0 || slot->search.query[0] == '\0') ? t->text_muted : t->danger;
        font_draw_text(g_ui.font_ui, count_buf, 0, count_x, bar_y + (bar_h - count_fh) * 0.5f, count_col);

        // Previous button ▲
        float btn_x = bar_x + bar_w - 92.0f;
        Rect prev_rect = rect_make(btn_x, bar_y + 4.0f, 20.0f, 24.0f);
        if (ui_button_ghost("\xe2\x96\xb2", prev_rect)) { // ▲
            jump_prev = true;
        }

        // Next button ▼
        Rect next_rect = rect_make(btn_x + 22.0f, bar_y + 4.0f, 20.0f, 24.0f);
        if (ui_button_ghost("\xe2\x96\xbc", next_rect)) { // ▼
            jump_next = true;
        }

        // Case sensitive toggle button [Aa]
        Rect case_rect = rect_make(btn_x + 44.0f, bar_y + 4.0f, 24.0f, 24.0f);
        if (slot->search.case_sensitive) {
            r_draw_rect_rounded(case_rect, 3.0f, color_with_alpha(t->primary, 0.35f));
        }
        if (ui_button_ghost("Aa", case_rect)) {
            slot->search.case_sensitive = !slot->search.case_sensitive;
            slot->search.matches_dirty = true;
        }

        // Close button ×
        Rect close_rect = rect_make(btn_x + 70.0f, bar_y + 4.0f, 18.0f, 24.0f);
        if (ui_button_ghost("\xc3\x97", close_rect)) {
            slot->search.is_open = false;
            g_ui.focus_id = id;
        }

        // Handle jumping
        if (slot->search.match_count > 0 && slot->search.matches) {
            if (jump_next) {
                slot->search.active_match_idx = (slot->search.active_match_idx + 1) % (int)slot->search.match_count;
                TextRange* m = &slot->search.matches[slot->search.active_match_idx];
                g_ui.sel_start = m->start;
                g_ui.sel_end = m->start + m->length;
                g_ui.editor_sel_id = id;

                // Smoothly center in viewport
                float rel_cx = 0.0f, rel_cy = 0.0f;
                font_layout_get_cursor_pos(layout, g_ui.font_mono, text, m->start, &rel_cx, &rel_cy, &line_h);
                sp->target_y = rel_cy - rect.h * 0.4f;
                if (sp->target_y < 0.0f) sp->target_y = 0.0f;
                if (sp->target_y > sp->max_scroll) sp->target_y = sp->max_scroll;
                sp->alpha = 1.0f;
                sp->fade_timer = 1.5f;
            } else if (jump_prev) {
                slot->search.active_match_idx = (slot->search.active_match_idx - 1 + (int)slot->search.match_count) % (int)slot->search.match_count;
                TextRange* m = &slot->search.matches[slot->search.active_match_idx];
                g_ui.sel_start = m->start;
                g_ui.sel_end = m->start + m->length;
                g_ui.editor_sel_id = id;

                // Smoothly center in viewport
                float rel_cx = 0.0f, rel_cy = 0.0f;
                font_layout_get_cursor_pos(layout, g_ui.font_mono, text, m->start, &rel_cx, &rel_cy, &line_h);
                sp->target_y = rel_cy - rect.h * 0.4f;
                if (sp->target_y < 0.0f) sp->target_y = 0.0f;
                if (sp->target_y > sp->max_scroll) sp->target_y = sp->max_scroll;
                sp->alpha = 1.0f;
                sp->fade_timer = 1.5f;
            }
        }
    }

    r_pop_clip();
    ui_draw_scrollbar(id, rect, total_content_h);
    return changed;
}

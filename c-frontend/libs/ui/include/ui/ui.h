#ifndef UI_H
#define UI_H

#include "base/base.h"
#include "ui/theme.h"
#include "text/font.h"
#include "icons/icon.h"

typedef struct {
    float mouse_x, mouse_y;
    float prev_mouse_x, prev_mouse_y;
    bool mouse_down[3];       // 0: left, 1: right, 2: middle
    bool mouse_clicked[3];
    bool mouse_released[3];
    int mouse_click_count;
    float scroll_x, scroll_y;
    
    // Keyboard
    bool ctrl_down;
    bool shift_down;
    bool alt_down;
    int key_pressed;
    char char_input[32];
    size_t char_input_len;

    // Active Widget State
    uint64_t hover_id;
    uint64_t active_id;
    uint64_t focus_id;

    // Text Editor Selection & Clipboard
    uint64_t editor_sel_id;
    size_t sel_start;
    size_t sel_end;
    bool is_selecting;

    // Open Modals / Menus
    uint64_t active_dropdown_id;
    uint64_t active_context_menu_id;
    Vec2 context_menu_pos;

    // Fonts
    Font* font_ui;
    Font* font_mono;
    Font* font_bold;

    // Window dimensions
    int window_width;
    int window_height;
    float dt;
} UIContext;

extern UIContext g_ui;

// Font sizes are logical pixels; a NULL or empty family falls back to the
// system default for that role. Nothing about typography is hardcoded here.
typedef struct {
    float size_ui, size_mono, size_bold;
    const char* family_ui;
    const char* family_mono;
} UiFontSpec;

void ui_init(int window_w, int window_h, UiFontSpec fonts);
void ui_shutdown(void);
void ui_begin_frame(float dt);
void ui_end_frame(void);

// ID Generation
uint64_t ui_hash_id(const char* str, uint64_t seed);

// Flexbox & Auto-Layout Engine
typedef struct {
    Rect container;
    float gap;
    float pad_x;
    float pad_y;
    float cur_x;
    float cur_y;
    float max_x;
    float max_y;
    bool is_row;
} UILayout;

UILayout ui_layout_row(Rect container, float gap, float pad_x, float pad_y);
UILayout ui_layout_col(Rect container, float gap, float pad_x, float pad_y);
Rect ui_layout_item_fixed(UILayout* l, float main_size, float cross_size);
Rect ui_layout_item_flex(UILayout* l, float flex_w, float min_w, float cross_size);
Rect ui_layout_item_right(UILayout* l, float width, float height);
bool ui_layout_can_fit(const UILayout* l, float required_size);
float ui_layout_remaining_space(const UILayout* l);

// Drawing & Layout Primitives
bool ui_button(const char* label, Rect rect, bool is_primary);
bool ui_button_ghost(const char* label, Rect rect);
bool ui_button_danger(const char* label, Rect rect);
bool ui_button_icon(IconType icon, const char* label, Rect rect, bool is_active);
bool ui_button_primary_icon(IconType icon, const char* label, Rect rect);
bool ui_button_danger_icon(IconType icon, const char* label, Rect rect);
bool ui_checkbox(const char* label, bool* checked, Rect rect);

bool ui_input_text(const char* id_str, char* buffer, size_t buffer_size, const char* placeholder, Rect rect);
bool ui_input_text_icon(const char* id_str, IconType icon, char* buffer, size_t buffer_size, const char* placeholder, Rect rect);

// Immediate-mode code editor. Superseded by libs/ui/components/code_editor.c,
// which virtualizes lines and takes a tokenizer callback rather than a hardcoded
// JSON flag. Kept until request_view.c and response_view.c stop drawing their
// bodies immediately; it lives here because it depends on this file's editor
// slot machinery, not because it belongs in a widget library.
bool ui_code_editor_legacy(const char* id_str, char** content_ptr, size_t* content_len,
                           size_t* content_cap, bool show_line_numbers,
                           bool is_json_syntax, bool is_readonly, Rect rect,
                           float* scroll_y);
void ui_code_editor_open_search_legacy(const char* id_str);

// Tabs & Navigation
int ui_tab_bar(const char* id_str, const char** tab_labels, int tab_count, int active_index, Rect rect);
int ui_tab_bar_icons(const char* id_str, const IconType* icons, const char** tab_labels, int tab_count, int active_index, Rect rect);

// Dropdowns & Menus
bool ui_dropdown(const char* id_str, const char** items, int item_count, int* selected_index, Rect rect);


// Splitter / Resizable Panels
bool ui_splitter(const char* id_str, float* split_pos, bool is_vertical, float min_a, float min_b, Rect container);


// Data Table for JSON responses
bool ui_data_table(const char* id_str, const char** columns, size_t col_count,
                   char*** rows, size_t row_count, const char* filter_text,
                   int* selected_row, int* selected_col, Rect rect, float* scroll_y);

// Modal & Overlay
bool ui_modal_begin(const char* title, float w, float h, Rect* out_body_rect);
void ui_modal_end(void);

// Context menu popup
bool ui_context_menu_begin(const char* id_str, float w, float* out_y);
bool ui_context_menu_item(IconType icon, const char* label, float w, float* y_cursor);
void ui_context_menu_end(void);

// Tooltip
void ui_set_tooltip(const char* text);

// Modern Responsive Scrollbar & Kinetic Smooth Scroll Physics
float ui_smooth_scroll(uint64_t id, Rect container_rect, float content_height, float* inout_scroll_y);
void ui_draw_scrollbar(uint64_t id, Rect container_rect, float content_height);

#endif // UI_H

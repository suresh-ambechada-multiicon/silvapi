#include "views.h"
#include "gl_backend.h"
#include "panel_layout.h"

void view_top_bar(Rect rect) {
    Theme* t = theme_get_active();

    // Background
    r_draw_rect(rect, t->topbar_bg);
    r_draw_line((Vec2){rect.x, rect.y + rect.h}, (Vec2){rect.x + rect.w, rect.y + rect.h}, 1.0f, t->border);

    float cur_x = rect.x + 12.0f;
    float btn_h = 24.0f;
    float by = rect.y + (rect.h - btn_h) * 0.5f;

    // Sidebar Toggle Button
    Rect toggle_btn = rect_make(cur_x, by, 26.0f, btn_h);
    if (ui_button_icon(g_layout.sidebar_collapsed ? ICON_EXPAND : ICON_COLLAPSE, NULL, toggle_btn, false)) {
        layout_toggle_sidebar();
    }
    cur_x += 32.0f;

    // Logo / Title
    font_draw_text(g_ui.font_bold, SILVAPI_TITLE, 0, cur_x, by + 4.0f, t->foreground);
    cur_x += font_measure_text(g_ui.font_bold, SILVAPI_TITLE, 0) + 20.0f;

    // Action buttons in Top Bar:
    // 1. Unified Import Button (Auto-detects OpenAPI, Postman, cURL, or URL)
    float b1_w = 82.0f;
    if (ui_button_icon(ICON_IMPORT, "Import", rect_make(cur_x, by, b1_w, btn_h), g_app.show_import_modal)) {
        g_app.show_import_modal = !g_app.show_import_modal;
        g_app.import_text_buffer[0] = '\0';
    }
    cur_x += b1_w + 6.0f;

    // 2. Variables Button
    float b2_w = 90.0f;
    if (ui_button_icon(ICON_VARIABLE, "Variables", rect_make(cur_x, by, b2_w, btn_h), g_app.show_variables_modal)) {
        g_app.show_variables_modal = !g_app.show_variables_modal;
    }
    cur_x += b2_w + 6.0f;

    // 3. Settings Button
    float b3_w = 85.0f;
    if (ui_button_icon(ICON_SETTINGS, "Settings", rect_make(cur_x, by, b3_w, btn_h), g_app.show_settings_modal)) {
        g_app.show_settings_modal = !g_app.show_settings_modal;
    }
    cur_x += b3_w + 6.0f;

    // 4. Quick Open Hint
    Rect search_hint_r = rect_make(cur_x + 12.0f, by, 180.0f, btn_h);
    if (ui_button_icon(ICON_SEARCH, "Search API (Ctrl+P)", search_hint_r, false)) {
        g_app.show_command_palette = true;
        g_app.command_palette_query[0] = '\0';
    }

    // Right Side: Window Control Badges
    float right_x = rect.x + rect.w - 70.0f;
    Rect min_rect = rect_make(right_x, by + 4.0f, 14.0f, 14.0f);
    Rect max_rect = rect_make(right_x + 20.0f, by + 4.0f, 14.0f, 14.0f);
    Rect close_rect = rect_make(right_x + 40.0f, by + 4.0f, 14.0f, 14.0f);

    r_draw_rect_rounded(min_rect, 7.0f, color_rgba(0.98f, 0.75f, 0.18f, 0.9f));
    r_draw_rect_rounded(max_rect, 7.0f, color_rgba(0.24f, 0.82f, 0.35f, 0.9f));
    r_draw_rect_rounded(close_rect, 7.0f, color_rgba(0.95f, 0.36f, 0.36f, 0.9f));
}

#include "views.h"
#include "gl_backend.h"
#include "panel_layout.h"

void view_top_bar(Rect rect) {
    Theme* t = theme_get_active();

    // Background
    r_draw_rect(rect, t->topbar_bg);
    r_draw_line((Vec2){rect.x, rect.y + rect.h}, (Vec2){rect.x + rect.w, rect.y + rect.h}, 1.0f, t->border);

    float btn_h = 24.0f;
    UILayout bar = ui_layout_row(rect, 6.0f, 12.0f, 12.0f);

    // Right Side: Window Control Badges (reserved first)
    Rect win_controls = ui_layout_item_right(&bar, 58.0f, btn_h);
    float dot_sz = 12.0f;
    float dot_y = win_controls.y + (win_controls.h - dot_sz) * 0.5f;
    Rect min_rect = rect_make(win_controls.x, dot_y, dot_sz, dot_sz);
    Rect max_rect = rect_make(win_controls.x + 18.0f, dot_y, dot_sz, dot_sz);
    Rect close_rect = rect_make(win_controls.x + 36.0f, dot_y, dot_sz, dot_sz);

    r_draw_rect_rounded(min_rect, 6.0f, color_rgba(0.98f, 0.75f, 0.18f, 0.9f));
    r_draw_rect_rounded(max_rect, 6.0f, color_rgba(0.24f, 0.82f, 0.35f, 0.9f));
    r_draw_rect_rounded(close_rect, 6.0f, color_rgba(0.95f, 0.36f, 0.36f, 0.9f));

    // Left Side Items:
    // 1. Sidebar Toggle Button
    Rect toggle_btn = ui_layout_item_fixed(&bar, 26.0f, btn_h);
    if (ui_button_icon(g_layout.sidebar_collapsed ? ICON_EXPAND : ICON_COLLAPSE, NULL, toggle_btn, false)) {
        layout_toggle_sidebar();
    }

    // 2. Logo / Title
    float isz = 16.0f;
    Rect logo_r = ui_layout_item_fixed(&bar, isz, isz);
    r_draw_icon(ICON_BOLT, logo_r, t->primary);

    float title_w = font_measure_text(g_ui.font_bold, SILVAPI_TITLE, 0);
    float title_h = font_get_height(g_ui.font_bold);
    Rect title_r = ui_layout_item_fixed(&bar, title_w + 12.0f, btn_h);
    font_draw_text(g_ui.font_bold, SILVAPI_TITLE, 0, title_r.x, title_r.y + (title_r.h - title_h) * 0.5f, t->foreground);

    // 3. Action buttons in Top Bar:
    // Import Button
    if (ui_layout_can_fit(&bar, 84.0f)) {
        Rect b1 = ui_layout_item_fixed(&bar, 84.0f, btn_h);
        if (ui_button_icon(ICON_IMPORT, "Import", b1, g_app.show_import_modal)) {
            g_app.show_import_modal = !g_app.show_import_modal;
            g_app.import_text_buffer[0] = '\0';
        }
    }

    // Variables Button
    if (ui_layout_can_fit(&bar, 94.0f)) {
        Rect b2 = ui_layout_item_fixed(&bar, 94.0f, btn_h);
        if (ui_button_icon(ICON_VARIABLE, "Variables", b2, g_app.show_variables_modal)) {
            g_app.show_variables_modal = !g_app.show_variables_modal;
        }
    }

    // Settings Button
    if (ui_layout_can_fit(&bar, 88.0f)) {
        Rect b3 = ui_layout_item_fixed(&bar, 88.0f, btn_h);
        if (ui_button_icon(ICON_SETTINGS, "Settings", b3, g_app.show_settings_modal)) {
            g_app.show_settings_modal = !g_app.show_settings_modal;
        }
    }

    // 4. Quick Open Search Hint (Fills remaining space nicely)
    if (ui_layout_remaining_space(&bar) > 120.0f) {
        float search_w = ui_layout_remaining_space(&bar);
        if (search_w > 260.0f) search_w = 260.0f;
        Rect search_hint_r = ui_layout_item_fixed(&bar, search_w, btn_h);
        if (ui_button_icon(ICON_SEARCH, "Search API (Ctrl+P)", search_hint_r, false)) {
            g_app.show_command_palette = true;
            g_app.command_palette_query[0] = '\0';
        }
    }
}

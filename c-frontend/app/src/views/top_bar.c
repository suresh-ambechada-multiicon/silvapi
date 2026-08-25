#include "app/views.h"
#include "gfx/gfx.h"
#include "ui/dock.h"
#include "ui/layout.h"

// ---------------------------------------------------------------------------
// Top bar, expressed declaratively.
//
// Previously this file computed every rect by hand: a UILayout cursor, explicit
// `ui_layout_item_fixed(&bar, 84.0f, btn_h)` widths per button, manual
// `ui_layout_can_fit()` guards, and `+ (title_r.h - title_h) * 0.5f` centring
// at each call site. The widths were magic numbers that had to be kept in sync
// with each label's text.
//
// Here sizing is UI_FIT (measure the content) or UI_GROW (take what's left),
// and vertical centring is a property of the row rather than arithmetic.
// ---------------------------------------------------------------------------

// Emits into the frame's single layout; it does not open one of its own.
// Clay keeps one global pointer-over set, rebuilt from the most recent
// Clay_EndLayout, so a second layout in the same frame destroys the first one's
// hit-test data and its buttons stop responding.
void view_top_bar(Rect rect) {
    Theme* t = theme_get_active();

    UI_ROW(.w = UI_GROW, .h = UI_PX(rect.h), .pad_x = 12, .gap = 6,
           .align_y = UI_CENTER, .bg = t->topbar_bg) {

        if (UI_BUTTON(.id = "topbar.sidebar_toggle",
                      .icon = g_layout.sidebar_collapsed ? ICON_EXPAND : ICON_COLLAPSE,
                      .fg = t->text_muted, .fg_hover = t->foreground,
                      .bg_hover = t->secondary_hover, .radius = 4, .pad_x = 5)) {
            layout_toggle_sidebar();
        }

        UI_ICON(ICON_BOLT, 16, t->primary);
        UI_LABEL(SILVAPI_TITLE, .color = t->foreground, .font = UI_FONT_BOLD);

        UI_SPACER(UI_PX(6));

        if (UI_BUTTON(.id = "topbar.import", .icon = ICON_IMPORT, .label = "Import",
                      .fg = t->text_muted, .fg_hover = t->foreground,
                      .bg = g_app.show_import_modal ? t->secondary_hover : (Color){0},
                      .bg_hover = t->secondary_hover, .radius = 4)) {
            g_app.show_import_modal = !g_app.show_import_modal;
            g_app.import_text_buffer[0] = '\0';
        }

        if (UI_BUTTON(.id = "topbar.variables", .icon = ICON_VARIABLE, .label = "Variables",
                      .fg = t->text_muted, .fg_hover = t->foreground,
                      .bg = g_app.show_variables_modal ? t->secondary_hover : (Color){0},
                      .bg_hover = t->secondary_hover, .radius = 4)) {
            g_app.show_variables_modal = !g_app.show_variables_modal;
        }

        if (UI_BUTTON(.id = "topbar.settings", .icon = ICON_SETTINGS, .label = "Settings",
                      .fg = t->text_muted, .fg_hover = t->foreground,
                      .bg = g_app.show_settings_modal ? t->secondary_hover : (Color){0},
                      .bg_hover = t->secondary_hover, .radius = 4)) {
            g_app.show_settings_modal = !g_app.show_settings_modal;
        }

        // Takes the leftover space, so the window controls stay pinned right
        // without the old "reserve the right side first" dance.
        UI_SPACER(UI_GROW);

        if (UI_BUTTON(.id = "topbar.search", .icon = ICON_SEARCH,
                      .label = "Search API (Ctrl+P)",
                      .fg = t->text_muted, .fg_hover = t->foreground,
                      .bg_hover = t->secondary_hover, .radius = 4)) {
            g_app.show_command_palette = true;
            g_app.command_palette_query[0] = '\0';
        }

        UI_SPACER(UI_GROW);

        // Window control badges.
        UI_ROW(.gap = 6, .align_y = UI_CENTER) {
            UI_BOX(.w = UI_PX(12), .h = UI_PX(12), .radius = 6,
                   .bg = color_rgba(0.98f, 0.75f, 0.18f, 0.9f)) {}
            UI_BOX(.w = UI_PX(12), .h = UI_PX(12), .radius = 6,
                   .bg = color_rgba(0.24f, 0.82f, 0.35f, 0.9f)) {}
            UI_BOX(.w = UI_PX(12), .h = UI_PX(12), .radius = 6,
                   .bg = color_rgba(0.95f, 0.36f, 0.36f, 0.9f)) {}
        }
    }

    // Hairline separator below the bar, as a layout row so it cannot drift.
    UI_BOX(.w = UI_GROW, .h = UI_PX(1.0f), .bg = t->border) {}
}

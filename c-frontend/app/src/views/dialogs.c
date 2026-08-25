#include "app/views.h"
#include "app/config.h"
#include "app/import_export.h"
#include "gfx/gfx.h"
#include "ui/dock.h"
#include "ui/layout.h"
#include "input/input.h"
#include "GLFW/glfw3.h"

// ---------------------------------------------------------------------------
// Dialogs, expressed declaratively.
//
// Every overlap bug in the previous version came from the same pattern: a
// hardcoded pixel offset standing in for a measured text width. Two examples
// that were visibly broken:
//
//   // theme buttons on a fixed 115px pitch with a 105px box:
//   Rect tr = rect_make(body_rect.x + p * 115.0f, sy, 105.0f, 28.0f);
//   //  -> "Catppuccin Mocha" is wider than 105px and spilled into its neighbour
//
//   font_draw_text(bold, "Dynamic Variables:", 0, body_rect.x, y, ...);
//   font_draw_text(ui,   "{{$guid}} ...",      0, body_rect.x + 130.0f, y, ...);
//   //  -> the bold label is wider than the 130px reserved for it
//
// Neither is expressible here. A UI_ROW sizes each child to its measured
// content and lays them out end to end, so siblings cannot collide. Overlap is
// reachable only by asking for it: .floating with an attach point.
// ---------------------------------------------------------------------------

// --- shared styling, all from config ---------------------------------------

typedef struct {
    Theme* t;
    float  radius;
    float  pad;
    float  gap;
    float  row_h;
    float  icon;
} Style;

static Style style_now(void) {
    Style s;
    s.t      = theme_get_active();
    s.radius = g_config.corner_radius;
    s.pad    = g_config.pad_large;
    s.gap    = g_config.gap_medium;
    s.row_h  = g_config.row_height;
    s.icon   = g_config.icon_size_small;
    return s;
}

static UiModalDesc modal_style(const Style* s, const char* id, float w, float h) {
    return (UiModalDesc){
        .id = id, .w = w, .h = h,
        .bg = s->t->panel_bg,
        .border = 1.0f, .border_color = s->t->border,
        .radius = s->radius + 2.0f,
        .scrim = color_rgba(0.0f, 0.0f, 0.0f, 0.45f),
        .pad = s->pad,
        .z = 100,
    };
}

// Title row with a close affordance; returns true if close was clicked.
static bool modal_header(const Style* s, const char* id, const char* title) {
    bool closed = false;
    UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h), .align_y = UI_CENTER, .gap = s->gap) {
        UI_LABEL(title, .color = s->t->foreground, .font = UI_FONT_BOLD);
        UI_SPACER(UI_GROW);
        char btn_id[96];
        snprintf(btn_id, sizeof(btn_id), "%s.close", id);
        if (UI_BUTTON(.id = btn_id, .icon = ICON_CLOSE,
                      .icon_size = s->icon,
                      .fg = s->t->text_muted, .fg_hover = s->t->foreground,
                      .bg_hover = s->t->secondary_hover,
                      .radius = s->radius, .pad_x = 5.0f,
                      .w = UI_PX(s->row_h - 4.0f), .h = UI_PX(s->row_h - 4.0f))) {
            closed = true;
        }
    }
    return closed;
}

static void section_label(const Style* s, const char* text) {
    UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h), .align_y = UI_CENTER) {
        UI_LABEL(text, .color = s->t->foreground, .font = UI_FONT_BOLD);
    }
}

// --- toast -----------------------------------------------------------------

static void view_toast(const Style* s) {
    if (g_app.toast_timer <= 0.0f) return;
    g_app.toast_timer -= g_ui.dt;

    // Floating on purpose: a toast is meant to sit above the content. It is
    // pinned bottom-centre by an attach point rather than by arithmetic on the
    // window size, so it stays centred as the window resizes.
    UI_ROW(.floating = true, .float_to_root = true,
           .float_attach = UI_ATTACH_BOTTOM_CENTER,
           .float_offset = { 0.0f, -40.0f },
           .float_z = 200,
           .h = UI_PX(34.0f), .pad_x = g_config.pad_medium, .gap = s->gap,
           .align_y = UI_CENTER,
           .bg = s->t->panel_bg, .radius = s->radius + 2.0f,
           .border = 1.0f, .border_color = s->t->primary) {
        UI_ICON(ICON_CHECK_CIRCLE, s->icon, s->t->primary);
        UI_LABEL(g_app.toast_message, .color = s->t->foreground);
    }
}

// --- variables modal -------------------------------------------------------

static void view_variables_modal(const Style* s) {
    if (!g_app.show_variables_modal) return;

    UI_MODAL_DESC(modal_style(s, "vars", 560.0f, 420.0f)) {
        if (modal_header(s, "vars", "Workspace Variables")) {
            g_app.show_variables_modal = false;
        }

        UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h), .align_y = UI_CENTER) {
            UI_LABEL("Manage workspace environment variables used with {{variable}} syntax:",
                     .color = s->t->text_muted);
        }

        Workspace* ws = g_app.workspace;
        if (ws) {
            // Column headers share the exact widths used by the rows below, so
            // the header and its column can never drift apart.
            const float check_w = 26.0f;
            const float del_w   = 26.0f;

            UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h), .gap = s->gap, .align_y = UI_CENTER) {
                UI_SPACER(UI_PX(check_w));
                UI_BOX(.w = UI_PCT(0.42f), .h = UI_GROW, .align_y = UI_CENTER) {
                    UI_LABEL("Key", .color = s->t->foreground, .font = UI_FONT_BOLD);
                }
                UI_BOX(.w = UI_GROW, .h = UI_GROW, .align_y = UI_CENTER) {
                    UI_LABEL("Value", .color = s->t->foreground, .font = UI_FONT_BOLD);
                }
                UI_SPACER(UI_PX(del_w));
            }

            int remove_at = -1;
            for (size_t i = 0; i < ws->variables_count; i++) {
                KeyValue* kv = &ws->variables[i];

                UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h + 4.0f), .gap = s->gap,
                       .align_y = UI_CENTER) {
                    UI_CHECKBOX(.id = "vars.on", .index = (int32_t)i, .value = &kv->enabled,
                                .size = 16.0f, .radius = 3.0f,
                                .bg = s->t->input_bg, .border_color = s->t->input_border,
                                .check_color = s->t->primary);

                    UI_BOX(.w = UI_PCT(0.42f), .h = UI_GROW, .align_y = UI_CENTER) {
                        if (UI_TEXT_INPUT(.id = "vars.key", .index = (int32_t)i,
                                          .buffer = kv->key, .buffer_size = sizeof(kv->key),
                                          .placeholder = "Key",
                                          .bg = s->t->input_bg, .fg = s->t->foreground,
                                          .placeholder_color = s->t->input_placeholder,
                                          .border_color = s->t->input_border,
                                          .border_focus_color = s->t->input_border_focus,
                                          .caret_color = s->t->primary,
                                          .radius = s->radius)) {
                            storage_save_workspace(ws);
                        }
                    }

                    UI_BOX(.w = UI_GROW, .h = UI_GROW, .align_y = UI_CENTER) {
                        if (UI_TEXT_INPUT(.id = "vars.val", .index = (int32_t)i,
                                          .buffer = kv->value, .buffer_size = sizeof(kv->value),
                                          .placeholder = "Value",
                                          .bg = s->t->input_bg, .fg = s->t->foreground,
                                          .placeholder_color = s->t->input_placeholder,
                                          .border_color = s->t->input_border,
                                          .border_focus_color = s->t->input_border_focus,
                                          .caret_color = s->t->primary,
                                          .radius = s->radius)) {
                            storage_save_workspace(ws);
                        }
                    }

                    if (UI_BUTTON(.id = "vars.del", .index = (int32_t)i, .icon = ICON_CLOSE,
                                  .icon_size = 12.0f,
                                  .w = UI_PX(del_w), .h = UI_PX(22.0f),
                                  .fg = s->t->text_muted, .fg_hover = s->t->danger,
                                  .bg_hover = s->t->secondary_hover, .radius = s->radius,
                                  .pad_x = 4.0f)) {
                        remove_at = (int)i;
                    }
                }
            }

            if (remove_at >= 0) {
                for (size_t k = (size_t)remove_at; k + 1 < ws->variables_count; k++) {
                    ws->variables[k] = ws->variables[k + 1];
                }
                if (ws->variables_count > 0) ws->variables_count--;
                storage_save_workspace(ws);
            }

            UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h), .align_y = UI_CENTER) {
                if (UI_BUTTON(.id = "vars.add", .icon = ICON_PLUS, .label = "Add",
                              .icon_size = 12.0f,
                              .fg = s->t->primary, .fg_hover = s->t->foreground,
                              .bg_hover = s->t->secondary_hover, .radius = s->radius)) {
                    kv_array_add(&ws->variables, &ws->variables_count, "", "", true);
                }
            }
        }

        UI_SPACER(UI_GROW);

        // The row that used to overlap. "Dynamic Variables:" is UI_FIT, so it
        // occupies exactly its measured width and the list follows after a gap.
        UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h), .gap = s->gap, .align_y = UI_CENTER) {
            UI_LABEL("Dynamic Variables:", .color = s->t->primary, .font = UI_FONT_BOLD);
            UI_LABEL("{{$guid}}  {{$timestamp}}  {{$randomInt}}", .color = s->t->text_muted);
        }

        UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h + 4.0f), .align_y = UI_CENTER) {
            UI_SPACER(UI_GROW);
            if (UI_BUTTON(.id = "vars.done", .label = "Done",
                          .w = UI_PX(84.0f), .h = UI_PX(26.0f),
                          .fg = s->t->primary_text, .bg = s->t->primary,
                          .bg_hover = s->t->primary_hover, .radius = s->radius)) {
                g_app.show_variables_modal = false;
            }
        }
    }
}

// --- settings modal --------------------------------------------------------

static void view_settings_modal(const Style* s) {
    if (!g_app.show_settings_modal) return;

    UI_MODAL_DESC(modal_style(s, "settings", 520.0f, 380.0f)) {
        if (modal_header(s, "settings", "Settings & Appearance")) {
            g_app.show_settings_modal = false;
        }

        section_label(s, "Theme Preset");

        // The row that used to overlap. Each button is UI_FIT, so it is exactly
        // as wide as its own label plus padding, whatever that label says.
        UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h + 4.0f), .gap = g_config.gap_small,
               .align_y = UI_CENTER) {
            for (int p = 0; p < THEME_COUNT; p++) {
                const char* name = theme_get_preset_name((ThemePreset)p);
                bool current = (theme_get_active()->name == name);

                if (UI_BUTTON(.id = "settings.theme", .index = p, .label = name,
                              .h = UI_PX(s->row_h),
                              .fg = current ? s->t->primary_text : s->t->text_muted,
                              .fg_hover = s->t->foreground,
                              .bg = current ? s->t->primary : (Color){0},
                              .bg_hover = s->t->secondary_hover,
                              .radius = s->radius)) {
                    theme_set_preset((ThemePreset)p);
                    char pstr[16];
                    snprintf(pstr, sizeof(pstr), "%d", p);
                    storage_save_setting("theme_preset", pstr);
                }
            }
        }

        section_label(s, "Local Storage");
        UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h), .align_y = UI_CENTER) {
            UI_LABEL("SQLite Database: ~/.local/share/silvapi/silvapi.sqlite",
                     .color = s->t->text_muted);
        }

        if (g_config.source_path[0]) {
            UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h), .gap = s->gap, .align_y = UI_CENTER) {
                UI_LABEL("Config:", .color = s->t->foreground, .font = UI_FONT_BOLD);
                UI_LABEL(g_config.source_path, .color = s->t->text_muted);
            }
        }

        UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h + 6.0f), .align_y = UI_CENTER) {
            if (UI_BUTTON(.id = "settings.reset_layout", .icon = ICON_REFRESH,
                          .label = "Reset Layout", .icon_size = s->icon,
                          .h = UI_PX(s->row_h),
                          .fg = s->t->text_muted, .fg_hover = s->t->foreground,
                          .bg_hover = s->t->secondary_hover, .radius = s->radius)) {
                layout_reset_default((float)g_ui.window_width, (float)g_ui.window_height);
                app_show_toast("Panel layout reset to default");
            }
        }

        UI_SPACER(UI_GROW);

        UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h + 4.0f), .align_y = UI_CENTER) {
            UI_SPACER(UI_GROW);
            if (UI_BUTTON(.id = "settings.done", .label = "Close",
                          .w = UI_PX(84.0f), .h = UI_PX(26.0f),
                          .fg = s->t->primary_text, .bg = s->t->primary,
                          .bg_hover = s->t->primary_hover, .radius = s->radius)) {
                g_app.show_settings_modal = false;
            }
        }
    }
}

// --- command palette -------------------------------------------------------

static void view_command_palette(const Style* s) {
    if (!g_app.show_command_palette) return;

    UI_MODAL_DESC(modal_style(s, "palette", 620.0f, 400.0f)) {
        if (modal_header(s, "palette", "Quick Open (Ctrl+P)")) {
            g_app.show_command_palette = false;
        }

        UI_ROW(.w = UI_GROW, .h = UI_PX(34.0f), .gap = s->gap, .align_y = UI_CENTER) {
            UI_TEXT_INPUT(.id = "palette.query",
                          .buffer = g_app.command_palette_query,
                          .buffer_size = sizeof(g_app.command_palette_query),
                          .placeholder = "Search requests by name or URL...",
                          .h = UI_PX(32.0f),
                          .bg = s->t->input_bg, .fg = s->t->foreground,
                          .placeholder_color = s->t->input_placeholder,
                          .border_color = s->t->input_border,
                          .border_focus_color = s->t->input_border_focus,
                          .caret_color = s->t->primary,
                          .radius = s->radius);
        }

        Workspace* ws = g_app.workspace;
        int match = 0;
        if (ws) {
            for (size_t c = 0; c < ws->collections_count && match <= 8; c++) {
                Collection* col = ws->collections[c];
                if (!col) continue;

                for (size_t i = 0; i < col->items_count && match <= 8; i++) {
                    CollectionItem* ci = col->items[i];
                    if (!ci || ci->type != ITEM_REQUEST || !ci->request) continue;

                    ApiRequest* req = ci->request;
                    if (g_app.command_palette_query[0] != '\0' &&
                        !strcasestr(req->name, g_app.command_palette_query) &&
                        !strcasestr(req->url, g_app.command_palette_query)) {
                        continue;
                    }

                    bool selected = (g_app.command_palette_sel_idx == match);
                    const char* mstr = http_method_to_string(req->method);
                    Color mcol = theme_get_method_color(mstr);

                    char row_id[64];
                    snprintf(row_id, sizeof(row_id), "palette.row");
                    bool hovered = uil_element_hovered(row_id, match);

                    UI_ROW(.id = row_id, .index = match,
                           .w = UI_GROW, .h = UI_PX(32.0f), .gap = s->gap,
                           .pad_x = g_config.pad_medium, .align_y = UI_CENTER,
                           .radius = s->radius,
                           .bg = selected ? s->t->secondary_active
                                          : (hovered ? s->t->secondary_hover : (Color){0})) {
                        // Method badge: sized to its own text, so a longer
                        // method like OPTIONS pushes the name across instead of
                        // being drawn over it.
                        UI_ROW(.h = UI_PX(22.0f), .pad_x = 6.0f, .align_y = UI_CENTER,
                               .radius = 3.0f, .bg = color_with_alpha(mcol, 0.15f)) {
                            UI_LABEL(mstr, .color = mcol, .font = UI_FONT_BOLD);
                        }

                        char crumb[512];
                        snprintf(crumb, sizeof(crumb), "%s  /  %s", col->name, req->name);
                        UI_LABEL(crumb, .color = s->t->foreground);
                    }

                    if ((hovered && input_mouse_pressed(INPUT_MOUSE_LEFT)) ||
                        (selected && input_key_pressed(INPUT_KEY_ENTER))) {
                        app_open_request(req);
                        g_app.show_command_palette = false;
                    }
                    match++;
                }
            }
        }

        if (match == 0) {
            UI_ROW(.w = UI_GROW, .h = UI_PX(32.0f), .align_y = UI_CENTER) {
                UI_LABEL("No matching requests", .color = s->t->text_muted);
            }
        }
    }
}

// --- import modal ----------------------------------------------------------

static void view_import_modal(const Style* s) {
    if (!g_app.show_import_modal) return;

    UI_MODAL_DESC(modal_style(s, "import", 640.0f, 460.0f)) {
        if (modal_header(s, "import", "Import API / Collection / cURL")) {
            g_app.show_import_modal = false;
        }

        UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h), .align_y = UI_CENTER) {
            UI_LABEL("Paste OpenAPI 3.0 JSON, Postman Collection JSON, a cURL command, or a URL:",
                     .color = s->t->text_muted);
        }

        UI_BOX(.w = UI_GROW, .h = UI_GROW) {
            UI_TEXT_INPUT(.id = "import.text",
                          .buffer = g_app.import_text_buffer,
                          .buffer_size = sizeof(g_app.import_text_buffer),
                          .placeholder = "Paste JSON or cURL string here...",
                          .h = UI_GROW,
                          .bg = s->t->input_bg, .fg = s->t->foreground,
                          .placeholder_color = s->t->input_placeholder,
                          .border_color = s->t->input_border,
                          .border_focus_color = s->t->input_border_focus,
                          .caret_color = s->t->primary,
                          .radius = s->radius);
        }

        char detected[64] = "Waiting for input...";
        if (g_app.import_text_buffer[0] != '\0') {
            import_auto_detect(g_app.import_text_buffer, detected, sizeof(detected));
        }

        UI_ROW(.w = UI_GROW, .h = UI_PX(s->row_h + 6.0f), .gap = s->gap, .align_y = UI_CENTER) {
            UI_LABEL("Detected:", .color = s->t->primary, .font = UI_FONT_BOLD);
            UI_LABEL(detected, .color = s->t->foreground);
            UI_SPACER(UI_GROW);

            if (UI_BUTTON(.id = "import.go", .icon = ICON_IMPORT, .label = "Import",
                          .icon_size = s->icon, .h = UI_PX(30.0f),
                          .fg = s->t->primary_text, .bg = s->t->primary,
                          .bg_hover = s->t->primary_hover, .radius = s->radius)) {
                if (g_app.import_text_buffer[0] != '\0' && g_app.workspace) {
                    char fmt[64];
                    Collection* col = import_auto_detect(g_app.import_text_buffer,
                                                         fmt, sizeof(fmt));
                    if (col) {
                        Workspace* ws = g_app.workspace;
                        Collection** grown = realloc(ws->collections,
                                                     sizeof(Collection*) * (ws->collections_count + 1));
                        if (grown) {
                            ws->collections = grown;
                            ws->collections[ws->collections_count++] = col;
                            storage_save_workspace(ws);

                            if (col->items_count > 0 && col->items[0]->type == ITEM_REQUEST) {
                                app_open_request(col->items[0]->request);
                            }
                            char msg[128];
                            snprintf(msg, sizeof(msg), "Imported %s successfully", fmt);
                            app_show_toast(msg);
                        }
                    } else {
                        app_show_toast("Failed to parse imported content");
                    }
                }
                g_app.show_import_modal = false;
            }
        }
    }
}

// --- context menu ----------------------------------------------------------

static void ctx_new_request(void) {
    ApiRequest* nr = api_request_create("New Request", METHOD_GET,
                                        "https://jsonplaceholder.typicode.com/posts");
    CollectionItem* ci = calloc(1, sizeof(CollectionItem));
    if (!ci) return;
    ci->type = ITEM_REQUEST;
    ci->request = nr;

    if (g_app.workspace && g_app.workspace->collections_count > 0) {
        Collection* col = g_app.workspace->collections[0];
        CollectionItem** grown = realloc(col->items,
                                         sizeof(CollectionItem*) * (col->items_count + 1));
        if (grown) {
            col->items = grown;
            col->items[col->items_count++] = ci;
            storage_save_workspace(g_app.workspace);
            app_open_request(nr);
        }
    }
}

static void ctx_new_folder(void) {
    Folder* nf = folder_create("New Folder");
    CollectionItem* ci = calloc(1, sizeof(CollectionItem));
    if (!ci) return;
    ci->type = ITEM_FOLDER;
    ci->folder = nf;

    if (g_app.workspace && g_app.workspace->collections_count > 0) {
        Collection* col = g_app.workspace->collections[0];
        CollectionItem** grown = realloc(col->items,
                                         sizeof(CollectionItem*) * (col->items_count + 1));
        if (grown) {
            col->items = grown;
            col->items[col->items_count++] = ci;
            storage_save_workspace(g_app.workspace);
        }
    }
}

static void ctx_copy_curl(void) {
    if (g_app.tab_count == 0 || g_app.active_tab_index < 0) return;
    ApiRequest* r = g_app.open_requests[g_app.active_tab_index];
    char* cmd = export_curl_command(r, g_app.workspace, NULL, NULL);
    if (!cmd) return;
    glfwSetClipboardString(NULL, cmd);
    free(cmd);
    app_show_toast("cURL command copied to clipboard");
}

static void view_context_menu(const Style* s) {
    if (!g_app.show_context_menu) return;

    // Anchored to an absolute point on purpose: a context menu belongs at the
    // pointer, over whatever is beneath it.
    UI_COL(.floating = true, .float_to_root = true,
           .float_attach = UI_ATTACH_TOP_LEFT,
           .float_offset = { g_ui.context_menu_pos.x, g_ui.context_menu_pos.y },
           .float_z = 300,
           .w = UI_PX(180.0f), .pad = g_config.pad_small, .gap = 1.0f,
           .bg = s->t->panel_bg, .radius = s->radius,
           .border = 1.0f, .border_color = s->t->border) {

        static const struct { IconType icon; const char* label; int action; } items[] = {
            { ICON_PLUS,   "New Request", 0 },
            { ICON_FOLDER, "New Folder",  1 },
            { ICON_COPY,   "Copy cURL",   2 },
            { ICON_TRASH,  "Delete",      3 },
        };

        for (size_t i = 0; i < sizeof(items) / sizeof(items[0]); i++) {
            bool danger = (items[i].action == 3);
            if (UI_BUTTON(.id = "ctx.item", .index = (int32_t)i,
                          .icon = items[i].icon, .label = items[i].label,
                          .icon_size = s->icon,
                          .w = UI_GROW, .h = UI_PX(s->row_h),
                          .pad_x = g_config.pad_medium,
                          .fg = danger ? s->t->danger : s->t->foreground,
                          .fg_hover = danger ? s->t->danger_hover : s->t->foreground,
                          .bg_hover = s->t->secondary_hover,
                          .radius = s->radius)) {
                switch (items[i].action) {
                    case 0: ctx_new_request(); break;
                    case 1: ctx_new_folder();  break;
                    case 2: ctx_copy_curl();   break;
                    default: break;
                }
                g_app.show_context_menu = false;
            }
        }
    }
}

// ---------------------------------------------------------------------------

// Emits into the frame's single layout. Every dialog is floating and attached to
// the layout root, so they overlay whatever is beneath them regardless of where
// in the tree this is called from.
void view_dialogs(void) {
    Style s = style_now();

    view_toast(&s);
    view_variables_modal(&s);
    view_settings_modal(&s);
    view_command_palette(&s);
    view_import_modal(&s);
    view_context_menu(&s);

    // Escape closes whatever is open, and a click outside the menu dismisses it.
    if (input_key_pressed(INPUT_KEY_ESCAPE)) {
        g_app.show_variables_modal = false;
        g_app.show_settings_modal = false;
        g_app.show_command_palette = false;
        g_app.show_import_modal = false;
        g_app.show_context_menu = false;
    }
}

#include "views.h"
#include "gl_backend.h"
#include "import_export.h"
#include "panel_layout.h"
#include "GLFW/glfw3.h"

void view_dialogs(void) {
    Theme* t = theme_get_active();

    // 1. Toast Notification Overlay
    if (g_app.toast_timer > 0.0f) {
        g_app.toast_timer -= g_ui.dt;
        float tw = font_measure_text(g_ui.font_ui, g_app.toast_message, 0) + 42.0f;
        float th = 34.0f;
        float tx = ((float)g_ui.window_width - tw) * 0.5f;
        float ty = (float)g_ui.window_height - 60.0f;

        Rect tr = rect_make(tx, ty, tw, th);
        r_draw_rect_rounded_bordered(tr, 6.0f, 1.0f, t->primary, t->panel_bg);
        r_draw_icon(ICON_CHECK_CIRCLE, rect_make(tx + 10.0f, ty + 9.0f, 16.0f, 16.0f), t->primary);
        float fh_ui = font_get_height(g_ui.font_ui);
        font_draw_text(g_ui.font_ui, g_app.toast_message, 0, tx + 32.0f, ty + (th - fh_ui) * 0.5f, t->foreground);
    }

    // 2. Variables Modal
    if (g_app.show_variables_modal) {
        Rect body_rect;
        if (ui_modal_begin("Workspace Variables", 550.0f, 400.0f, &body_rect)) {
            font_draw_text(g_ui.font_ui, "Manage workspace environment variables used with {{variable}} syntax:", 0, body_rect.x, body_rect.y, t->text_muted);

            Rect kv_rect = rect_make(body_rect.x, body_rect.y + 26.0f, body_rect.w, body_rect.h - 90.0f);
            if (g_app.workspace) {
                if (ui_kv_editor("ws_vars", &g_app.workspace->variables, &g_app.workspace->variables_count, kv_rect, NULL)) {
                    storage_save_workspace(g_app.workspace);
                }
            }

            // Dynamic variable hint
            float hint_y = body_rect.y + body_rect.h - 55.0f;
            font_draw_text(g_ui.font_bold, "Dynamic Variables:", 0, body_rect.x, hint_y, t->primary);
            font_draw_text(g_ui.font_ui, "{{$guid}}  {{$timestamp}}  {{$randomInt}}", 0, body_rect.x + 130.0f, hint_y, t->text_muted);

            // Close Button
            Rect close_btn = rect_make(body_rect.x + body_rect.w - 80.0f, body_rect.y + body_rect.h - 30.0f, 80.0f, 26.0f);
            if (ui_button("Done", close_btn, true)) {
                g_app.show_variables_modal = false;
            }

            ui_modal_end();
        } else {
            g_app.show_variables_modal = false;
        }
    }

    // 3. Settings Modal
    if (g_app.show_settings_modal) {
        Rect body_rect;
        if (ui_modal_begin("Settings & Appearance", 500.0f, 360.0f, &body_rect)) {
            float sy = body_rect.y;

            // Theme Selection
            font_draw_text(g_ui.font_bold, "Theme Preset", 0, body_rect.x, sy, t->foreground);
            sy += 24.0f;

            for (int p = 0; p < THEME_COUNT; p++) {
                Rect tr = rect_make(body_rect.x + p * 115.0f, sy, 105.0f, 28.0f);
                bool is_cur = (theme_get_active()->name == theme_get_preset_name((ThemePreset)p));
                if (ui_button_icon(ICON_NONE, theme_get_preset_name((ThemePreset)p), tr, is_cur)) {
                    theme_set_preset((ThemePreset)p);
                    char pstr[16];
                    snprintf(pstr, sizeof(pstr), "%d", p);
                    storage_save_setting("theme_preset", pstr);
                }
            }
            sy += 48.0f;

            // Storage DB Info
            font_draw_text(g_ui.font_bold, "Local Storage", 0, body_rect.x, sy, t->foreground);
            sy += 24.0f;
            font_draw_text(g_ui.font_ui, "SQLite Database: ~/.local/share/silvapi/silvapi.sqlite", 0, body_rect.x, sy, t->text_muted);
            sy += 40.0f;

            // Reset Layout button
            Rect rst_btn = rect_make(body_rect.x, sy, 130.0f, 26.0f);
            if (ui_button_ghost("Reset Layout", rst_btn)) {
                layout_reset_default((float)g_ui.window_width, (float)g_ui.window_height);
                app_show_toast("Panel layout reset to default");
            }

            // Done Button
            Rect done_btn = rect_make(body_rect.x + body_rect.w - 80.0f, body_rect.y + body_rect.h - 30.0f, 80.0f, 26.0f);
            if (ui_button("Close", done_btn, true)) {
                g_app.show_settings_modal = false;
            }

            ui_modal_end();
        } else {
            g_app.show_settings_modal = false;
        }
    }

    // 4. Command Palette (Search API / Ctrl+P modal)
    if (g_app.show_command_palette) {
        float pw = 600.0f;
        float ph = 380.0f;
        Rect body_rect;
        if (ui_modal_begin("Quick Open (Ctrl+P)", pw, ph, &body_rect)) {
            // Search Input
            Rect search_r = rect_make(body_rect.x, body_rect.y, body_rect.w, 32.0f);
            ui_input_text_icon("cmd_palette_input", ICON_SEARCH, g_app.command_palette_query, sizeof(g_app.command_palette_query), "Search requests by name or URL...", search_r);

            // List Results
            float list_y = body_rect.y + 40.0f;
            float item_h = 32.0f;
            int match_idx = 0;

            if (g_app.workspace) {
                for (size_t c = 0; c < g_app.workspace->collections_count; c++) {
                    Collection* col = g_app.workspace->collections[c];
                    if (!col) continue;

                    for (size_t i = 0; i < col->items_count; i++) {
                        CollectionItem* ci = col->items[i];
                        if (ci->type == ITEM_REQUEST && ci->request) {
                            ApiRequest* req = ci->request;
                            if (g_app.command_palette_query[0] != '\0') {
                                if (!strcasestr(req->name, g_app.command_palette_query) &&
                                    !strcasestr(req->url, g_app.command_palette_query)) {
                                    continue;
                                }
                            }

                            Rect ir = rect_make(body_rect.x, list_y, body_rect.w, item_h);
                            bool hov = rect_contains(ir, g_ui.mouse_x, g_ui.mouse_y);
                            bool is_sel = (g_app.command_palette_sel_idx == match_idx);

                            if (hov || is_sel) {
                                r_draw_rect_rounded(ir, 4.0f, is_sel ? t->secondary_active : t->secondary_hover);
                            }

                            if ((hov && g_ui.mouse_clicked[0]) || (is_sel && g_ui.key_pressed == GLFW_KEY_ENTER)) {
                                app_open_request(req);
                                g_app.show_command_palette = false;
                                break;
                            }

                            // Method Badge
                            const char* mstr = http_method_to_string(req->method);
                            Color mcol = theme_get_method_color(mstr);
                            float fh_bold = font_get_height(g_ui.font_bold);
                            float fh_ui = font_get_height(g_ui.font_ui);
                            float mw = font_measure_text(g_ui.font_bold, mstr, 0) + 12.0f;
                            float badge_h = 22.0f;
                            Rect badge_rect = rect_make(ir.x + 8.0f, ir.y + (item_h - badge_h) * 0.5f, mw, badge_h);
                            r_draw_rect_rounded(badge_rect, 3.0f, color_with_alpha(mcol, 0.15f));
                            font_draw_text(g_ui.font_bold, mstr, 0, badge_rect.x + 6.0f, badge_rect.y + (badge_h - fh_bold) * 0.5f, mcol);

                            // Breadcrumb
                            char breadcrumb[512];
                            snprintf(breadcrumb, sizeof(breadcrumb), "%s  /  %s", col->name, req->name);
                            font_draw_text(g_ui.font_ui, breadcrumb, 0, ir.x + mw + 16.0f, ir.y + (item_h - fh_ui) * 0.5f, t->foreground);

                            list_y += item_h + 2.0f;
                            match_idx++;
                            if (match_idx > 8) break;
                        }
                    }
                }
            }

            ui_modal_end();
        } else {
            g_app.show_command_palette = false;
        }
    }

    // 5. Unified Import Modal (Auto-detects OpenAPI, Postman, cURL, URL)
    if (g_app.show_import_modal) {
        Rect body_rect;
        if (ui_modal_begin("Import API / Collection / cURL", 620.0f, 440.0f, &body_rect)) {
            font_draw_text(g_ui.font_ui, "Paste raw OpenAPI 3.0 JSON, Postman Collection JSON, cURL command, or URL:", 0, body_rect.x, body_rect.y, t->text_muted);

            // Large text area
            Rect input_box = rect_make(body_rect.x, body_rect.y + 24.0f, body_rect.w, body_rect.h - 80.0f);
            ui_input_text("unified_import_input", g_app.import_text_buffer, sizeof(g_app.import_text_buffer), "Paste JSON or cURL string here...", input_box);

            // Format Detection Live Badge
            char detected_format[64] = "Waiting for input...";
            if (g_app.import_text_buffer[0] != '\0') {
                import_auto_detect(g_app.import_text_buffer, detected_format, sizeof(detected_format));
            }

            float by_bottom = body_rect.y + body_rect.h - 40.0f;
            float fh_bold = font_get_height(g_ui.font_bold);
            float fh_ui = font_get_height(g_ui.font_ui);
            font_draw_text(g_ui.font_bold, "Detected:", 0, body_rect.x, by_bottom + (30.0f - fh_bold) * 0.5f, t->primary);
            font_draw_text(g_ui.font_ui, detected_format, 0, body_rect.x + 80.0f, by_bottom + (30.0f - fh_ui) * 0.5f, t->foreground);

            // Import Button
            Rect imp_btn = rect_make(body_rect.x + body_rect.w - 100.0f, by_bottom, 100.0f, 30.0f);
            if (ui_button("Import", imp_btn, true)) {
                if (g_app.import_text_buffer[0] != '\0') {
                    char fmt[64];
                    Collection* col = import_auto_detect(g_app.import_text_buffer, fmt, sizeof(fmt));
                    if (col && g_app.workspace) {
                        g_app.workspace->collections = realloc(g_app.workspace->collections, sizeof(Collection*) * (g_app.workspace->collections_count + 1));
                        g_app.workspace->collections[g_app.workspace->collections_count++] = col;
                        storage_save_workspace(g_app.workspace);

                        // Open first request from imported collection
                        if (col->items_count > 0 && col->items[0]->type == ITEM_REQUEST) {
                            app_open_request(col->items[0]->request);
                        }

                        char toast_msg[128];
                        snprintf(toast_msg, sizeof(toast_msg), "Imported %s successfully", fmt);
                        app_show_toast(toast_msg);
                    } else {
                        app_show_toast("Failed to parse imported content");
                    }
                }
                g_app.show_import_modal = false;
            }

            ui_modal_end();
        } else {
            g_app.show_import_modal = false;
        }
    }

    // 6. Right-Click Context Menu for Sidebar
    if (g_app.show_context_menu) {
        float menu_y = 0;
        if (ui_context_menu_begin("sidebar_ctx", 160.0f, &menu_y)) {
            if (ui_context_menu_item(ICON_PLUS, "New Request", 160.0f, &menu_y)) {
                ApiRequest* nr = api_request_create("New Request", METHOD_GET, "https://jsonplaceholder.typicode.com/posts");
                CollectionItem* ci = calloc(1, sizeof(CollectionItem));
                ci->type = ITEM_REQUEST;
                ci->request = nr;
                if (g_app.workspace && g_app.workspace->collections_count > 0) {
                    Collection* col = g_app.workspace->collections[0];
                    col->items = realloc(col->items, sizeof(CollectionItem*) * (col->items_count + 1));
                    col->items[col->items_count++] = ci;
                    storage_save_workspace(g_app.workspace);
                    app_open_request(nr);
                }
                g_app.show_context_menu = false;
            }

            if (ui_context_menu_item(ICON_FOLDER, "New Folder", 160.0f, &menu_y)) {
                Folder* nf = folder_create("New Folder");
                CollectionItem* ci = calloc(1, sizeof(CollectionItem));
                ci->type = ITEM_FOLDER;
                ci->folder = nf;
                if (g_app.workspace && g_app.workspace->collections_count > 0) {
                    Collection* col = g_app.workspace->collections[0];
                    col->items = realloc(col->items, sizeof(CollectionItem*) * (col->items_count + 1));
                    col->items[col->items_count++] = ci;
                    storage_save_workspace(g_app.workspace);
                }
                g_app.show_context_menu = false;
            }

            if (ui_context_menu_item(ICON_COPY, "Copy cURL", 160.0f, &menu_y)) {
                if (g_app.tab_count > 0 && g_app.active_tab_index >= 0) {
                    ApiRequest* r = g_app.open_requests[g_app.active_tab_index];
                    char* curl_cmd = export_curl_command(r, g_app.workspace, NULL, NULL);
                    if (curl_cmd) {
                        glfwSetClipboardString(NULL, curl_cmd);
                        free(curl_cmd);
                        app_show_toast("cURL command copied to clipboard");
                    }
                }
                g_app.show_context_menu = false;
            }

            if (ui_context_menu_item(ICON_TRASH, "Delete", 160.0f, &menu_y)) {
                g_app.show_context_menu = false;
            }

            if (g_ui.mouse_clicked[0]) {
                g_app.show_context_menu = false;
            }
        }
    }
}

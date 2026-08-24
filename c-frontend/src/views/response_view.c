#include "views.h"
#include "gl_backend.h"
#include "GLFW/glfw3.h"

static void format_size(size_t bytes, char* out, size_t out_sz) {
    if (bytes < 1024) {
        snprintf(out, out_sz, "%zu B", bytes);
    } else if (bytes < 1024 * 1024) {
        snprintf(out, out_sz, "%.1f KB", (float)bytes / 1024.0f);
    } else {
        snprintf(out, out_sz, "%.2f MB", (float)bytes / (1024.0f * 1024.0f));
    }
}

static void format_duration(uint64_t ms, char* out, size_t out_sz) {
    if (ms < 1000) {
        snprintf(out, out_sz, "%lu ms", (unsigned long)ms);
    } else {
        snprintf(out, out_sz, "%.2f s", (float)ms / 1000.0f);
    }
}

void view_response_panel(Rect rect) {
    Theme* t = theme_get_active();

    r_draw_rect(rect, t->background);

    HttpResponse* resp = &g_app.active_response;
    bool has_response = (resp->status_code > 0 || resp->error != NULL);

    // 1. Status Bar Header
    float status_bar_h = 40.0f;
    Rect sb_rect = rect_make(rect.x, rect.y, rect.w, status_bar_h);
    r_draw_rect(sb_rect, t->tabbar_bg);
    r_draw_line((Vec2){rect.x, rect.y + status_bar_h}, (Vec2){rect.x + rect.w, rect.y + status_bar_h}, 1.0f, t->border);

    float fh_ui = font_get_height(g_ui.font_ui);
    float fh_bold = font_get_height(g_ui.font_bold);

    UILayout row = ui_layout_row(sb_rect, 8.0f, 10.0f, 8.0f);

    if (g_app.is_sending) {
        float isz = 14.0f;
        Rect ir = ui_layout_item_fixed(&row, isz, isz);
        r_draw_icon(ICON_REFRESH, ir, t->primary);
        font_draw_text(g_ui.font_bold, "Sending request...", 0, row.cur_x + 4.0f, rect.y + (status_bar_h - fh_bold) * 0.5f, t->primary);
    } else if (has_response) {
        // Reserve Action Buttons on Right first: Copy, Find, History
        Rect copy_btn = ui_layout_item_right(&row, 72.0f, 24.0f);
        if (ui_button_icon(ICON_COPY, "Copy", copy_btn, false)) {
            if (resp->body && *resp->body) {
                glfwSetClipboardString(NULL, resp->body);
                app_show_toast("Response copied to clipboard");
            }
        }

        // Find button (triggers fast search in response buffer)
        if (g_app.response_subtab == 0 && resp->body && *resp->body) {
            Rect find_btn = ui_layout_item_right(&row, 68.0f, 24.0f);
            if (ui_button_icon(ICON_SEARCH, "Find", find_btn, false)) {
                ui_code_editor_open_search("resp_code_viewer");
            }
        }

        Rect hist_btn = (Rect){0};
        if (ui_layout_remaining_space(&row) > 80.0f) {
            hist_btn = ui_layout_item_right(&row, 82.0f, 24.0f);
            if (ui_button_icon(ICON_HISTORY, "History", hist_btn, g_app.show_history_dropdown)) {
                g_app.show_history_dropdown = !g_app.show_history_dropdown;
                if (g_app.show_history_dropdown && g_app.tab_count > 0 && g_app.active_tab_index >= 0) {
                    if (g_app.history_entries) storage_free_history_entries(g_app.history_entries, g_app.history_count);
                    g_app.history_entries = storage_load_response_history(g_app.open_requests[g_app.active_tab_index]->id, &g_app.history_count);
                }
            }
        }

        if (resp->status_code > 0) {
            // Status Badge (e.g. "200 OK")
            char sbuf[64];
            snprintf(sbuf, sizeof(sbuf), "%d %.40s", resp->status_code, resp->status_text);
            Color scol = theme_get_status_color(resp->status_code);
            IconType sicon = (resp->status_code >= 200 && resp->status_code < 400) ? ICON_CHECK_CIRCLE : ICON_ALERT_TRIANGLE;
            float sw = font_measure_text(g_ui.font_bold, sbuf, 0) + 36.0f;
            
            if (ui_layout_remaining_space(&row) > 50.0f) {
                Rect s_badge = ui_layout_item_fixed(&row, sw, 24.0f);
                r_draw_rect_rounded(s_badge, 4.0f, color_with_alpha(scol, 0.2f));
                r_draw_icon(sicon, rect_make(s_badge.x + 8.0f, s_badge.y + 5.0f, 14.0f, 14.0f), scol);
                font_draw_text(g_ui.font_bold, sbuf, 0, s_badge.x + 26.0f, s_badge.y + (s_badge.h - fh_bold) * 0.5f, scol);
            }

            // Latency Badge
            char lbuf[32];
            format_duration(resp->latency_ms, lbuf, sizeof(lbuf));
            float lw = font_measure_text(g_ui.font_ui, lbuf, 0) + 32.0f;
            if (ui_layout_can_fit(&row, lw)) {
                Rect l_badge = ui_layout_item_fixed(&row, lw, 24.0f);
                r_draw_rect_rounded(l_badge, 4.0f, t->secondary);
                r_draw_icon(ICON_CLOCK, rect_make(l_badge.x + 7.0f, l_badge.y + 5.0f, 14.0f, 14.0f), t->text_muted);
                font_draw_text(g_ui.font_ui, lbuf, 0, l_badge.x + 24.0f, l_badge.y + (l_badge.h - fh_ui) * 0.5f, t->foreground);
            }

            // Size Badge
            char szbuf[32];
            format_size(resp->body_size, szbuf, sizeof(szbuf));
            float szw = font_measure_text(g_ui.font_ui, szbuf, 0) + 32.0f;
            if (ui_layout_can_fit(&row, szw)) {
                Rect sz_badge = ui_layout_item_fixed(&row, szw, 24.0f);
                r_draw_rect_rounded(sz_badge, 4.0f, t->secondary);
                r_draw_icon(ICON_DATABASE, rect_make(sz_badge.x + 7.0f, sz_badge.y + 5.0f, 14.0f, 14.0f), t->text_muted);
                font_draw_text(g_ui.font_ui, szbuf, 0, sz_badge.x + 24.0f, sz_badge.y + (sz_badge.h - fh_ui) * 0.5f, t->foreground);
            }
        } else if (resp->error) {
            float ew = font_measure_text(g_ui.font_bold, resp->error, 0) + 30.0f;
            Rect er = ui_layout_item_fixed(&row, ew, 24.0f);
            r_draw_icon(ICON_ALERT_TRIANGLE, rect_make(er.x, er.y + 4.0f, 16.0f, 16.0f), t->danger);
            font_draw_text(g_ui.font_bold, resp->error, 0, er.x + 22.0f, er.y + (er.h - fh_bold) * 0.5f, t->danger);
        }

        // Response History Dropdown Popup
        if (g_app.show_history_dropdown && hist_btn.w > 0.0f) {
            float hw = 300.0f;
            float hh = 200.0f;
            Rect hr = rect_make(hist_btn.x + hist_btn.w - hw, hist_btn.y + hist_btn.h + 4.0f, hw, hh);
            r_draw_rect_rounded_bordered(hr, 4.0f, 1.0f, t->border, t->panel_bg);

            // History header & Delete button
            Rect del_btn = rect_make(hr.x + 8.0f, hr.y + 6.0f, hr.w - 16.0f, 22.0f);
            char dlabel[64];
            snprintf(dlabel, sizeof(dlabel), "Delete %zu Responses", g_app.history_count);
            if (ui_button_danger(dlabel, del_btn)) {
                if (g_app.tab_count > 0 && g_app.active_tab_index >= 0) {
                    storage_delete_response_history(g_app.open_requests[g_app.active_tab_index]->id);
                    if (g_app.history_entries) {
                        storage_free_history_entries(g_app.history_entries, g_app.history_count);
                        g_app.history_entries = NULL;
                        g_app.history_count = 0;
                    }
                }
            }

            float hy = hr.y + 34.0f;
            for (size_t hi = 0; hi < g_app.history_count && hi < 6; hi++) {
                HistoryEntry* entry = &g_app.history_entries[hi];
                Rect item_r = rect_make(hr.x + 4.0f, hy, hr.w - 8.0f, 26.0f);
                bool hov = rect_contains(item_r, g_ui.mouse_x, g_ui.mouse_y);

                if (hov) {
                    r_draw_rect_rounded(item_r, 3.0f, t->secondary_hover);
                    if (g_ui.mouse_clicked[0]) {
                        // Restore snapshot
                        http_response_free_contents(&g_app.active_response);
                        g_app.active_response = (HttpResponse){0};
                        g_app.active_response.status_code = entry->status_code;
                        g_app.active_response.latency_ms = entry->latency_ms;
                        g_app.active_response.body_size = entry->size_bytes;
                        snprintf(g_app.active_response.status_text, sizeof(g_app.active_response.status_text), "%s", entry->response.status_text);
                        snprintf(g_app.active_response.timestamp, sizeof(g_app.active_response.timestamp), "%s", entry->timestamp);
                        if (entry->response.body) g_app.active_response.body = strdup(entry->response.body);
                        g_app.show_history_dropdown = false;
                        break;
                    }
                }

                char hline[128];
                snprintf(hline, sizeof(hline), "%s  %d  %lu ms", entry->timestamp, entry->status_code, (unsigned long)entry->latency_ms);
                Color hcol = theme_get_status_color(entry->status_code);
                font_draw_text(g_ui.font_ui, hline, 0, item_r.x + 6.0f, item_r.y + 6.0f, hov ? t->foreground : hcol);
                hy += 28.0f;
            }
        }
    } else {
        font_draw_text(g_ui.font_ui, "Enter a URL and click Send to see the response", 0, row.cur_x, rect.y + (status_bar_h - fh_ui) * 0.5f, t->text_muted);
    }

    // 2. Response Sub-tabs: Response, Table, Headers, Timeline with Modern Icons
    float subtab_y = rect.y + status_bar_h;
    float subtab_h = 28.0f;

    const char* subtabs[] = {
        "Response",
        "Table",
        "Headers",
        "Timeline"
    };

    const IconType subtab_icons[] = {
        ICON_CODE,
        ICON_TABLE,
        ICON_LAYERS,
        ICON_HISTORY
    };

    Rect subtab_bar = rect_make(rect.x + 8.0f, subtab_y, rect.w - 16.0f, subtab_h);
    g_app.response_subtab = ui_tab_bar_icons("resp_subtabs", subtab_icons, subtabs, 4, g_app.response_subtab, subtab_bar);

    // 3. Sub-tab Content Area
    float content_y = subtab_y + subtab_h + 4.0f;
    float content_h = rect.y + rect.h - content_y;
    Rect content_rect = rect_make(rect.x, content_y, rect.w, content_h);

    if (!has_response && !g_app.is_sending) {
        return;
    }

    switch (g_app.response_subtab) {
        case 0: { // Code / JSON Viewer Tab
            size_t blen = resp->body ? strlen(resp->body) : 0;
            size_t bcap = blen + 1;
            ui_code_editor("resp_code_viewer", &resp->body, &blen, &bcap,
                           true, true, true, content_rect, &g_app.response_body_scroll_y);
            break;
        }
        case 1: { // Table View Tab (with search, CSV export, and bottom detail inspector)
            if (!g_app.active_json_table && resp->body && *resp->body) {
                g_app.active_json_table = json_table_from_string(resp->body);
            }

            if (!g_app.active_json_table || g_app.active_json_table->row_count == 0) {
                font_draw_text(g_ui.font_ui, "Response body is not a JSON object or array", 0, content_rect.x + 20.0f, content_rect.y + 30.0f, t->text_muted);
                break;
            }

            // Top Bar of Table: Search Table + Row Count + Copy CSV Button
            float tbar_h = 32.0f;
            Rect tbar_rect = rect_make(content_rect.x + 8.0f, content_rect.y, content_rect.w - 16.0f, tbar_h);

            // Search input
            float tsearch_w = 200.0f;
            Rect tsearch_rect = rect_make(tbar_rect.x, tbar_rect.y + 2.0f, tsearch_w, 26.0f);
            ui_input_text_icon("table_search_input", ICON_SEARCH, g_app.table_search, sizeof(g_app.table_search), "Search table...", tsearch_rect);

            // Row count label
            char rcount_str[32];
            snprintf(rcount_str, sizeof(rcount_str), "%zu rows", g_app.active_json_table->row_count);
            font_draw_text(g_ui.font_ui, rcount_str, 0, tsearch_rect.x + tsearch_w + 12.0f, tbar_rect.y + 7.0f, t->text_muted);

            // Copy CSV Button
            float csv_w = 80.0f;
            Rect csv_rect = rect_make(tbar_rect.x + tbar_rect.w - csv_w, tbar_rect.y + 2.0f, csv_w, 26.0f);
            if (ui_button_icon(ICON_COPY, "CSV", csv_rect, false)) {
                char* csv_text = json_table_to_csv(g_app.active_json_table);
                if (csv_text) {
                    glfwSetClipboardString(NULL, csv_text);
                    free(csv_text);
                    app_show_toast("CSV copied to clipboard");
                }
            }

            // Table Grid Area
            float table_grid_y = content_rect.y + tbar_h + 4.0f;
            float detail_h = g_app.table_detail_height > 0 ? g_app.table_detail_height : 150.0f;
            float table_grid_h = content_rect.h - tbar_h - detail_h - 10.0f;
            if (table_grid_h < 100.0f) table_grid_h = 100.0f;

            Rect grid_rect = rect_make(content_rect.x, table_grid_y, content_rect.w, table_grid_h);
            ui_data_table("resp_data_grid", (const char**)g_app.active_json_table->columns, g_app.active_json_table->column_count,
                          g_app.active_json_table->rows, g_app.active_json_table->row_count, g_app.table_search,
                          &g_app.table_selected_row, &g_app.table_selected_col, grid_rect, &g_app.response_table_scroll_y);

            // Detail Inspector Splitter & Panel
            float detail_y = table_grid_y + table_grid_h;
            Rect detail_rect = rect_make(content_rect.x, detail_y, content_rect.w, content_rect.y + content_rect.h - detail_y);
            r_draw_rect(detail_rect, t->panel_bg);
            r_draw_line((Vec2){detail_rect.x, detail_rect.y}, (Vec2){detail_rect.x + detail_rect.w, detail_rect.y}, 1.0f, t->border);

            // Detail Header
            font_draw_text(g_ui.font_bold, "Cell Details", 0, detail_rect.x + 12.0f, detail_rect.y + 8.0f, t->foreground);

            // Markdown Toggle Button
            Rect md_btn = rect_make(detail_rect.x + detail_rect.w - 85.0f, detail_rect.y + 6.0f, 75.0f, 22.0f);
            if (ui_button_icon(ICON_NONE, "Markdown", md_btn, g_app.table_markdown_view)) {
                g_app.table_markdown_view = !g_app.table_markdown_view;
            }

            // Selected cell content
            if (g_app.table_selected_row >= 0 && g_app.table_selected_row < (int)g_app.active_json_table->row_count) {
                const char* cell_val = g_app.active_json_table->rows[g_app.table_selected_row][0];
                font_draw_text(g_ui.font_mono, cell_val, 0, detail_rect.x + 14.0f, detail_rect.y + 36.0f, t->foreground);
            }
            break;
        }
        case 2: { // Headers Tab
            float hy = content_rect.y + 10.0f;
            font_draw_text(g_ui.font_bold, "Response Headers", 0, content_rect.x + 16.0f, hy, t->foreground);
            hy += 26.0f;

            float key_col_w = 220.0f;
            for (size_t hi = 0; hi < resp->headers_count; hi++) {
                Rect hr = rect_make(content_rect.x + 16.0f, hy, content_rect.w - 32.0f, 26.0f);
                if (rect_contains(hr, g_ui.mouse_x, g_ui.mouse_y)) {
                    r_draw_rect_rounded(hr, 3.0f, t->secondary_hover);
                } else if (hi % 2 == 1) {
                    r_draw_rect_rounded(hr, 3.0f, color_with_alpha(t->secondary, 0.4f));
                }

                // Key column with clipping
                r_push_clip(rect_make(hr.x + 4.0f, hr.y, key_col_w - 8.0f, hr.h));
                font_draw_text(g_ui.font_bold, resp->headers[hi].key, 0, hr.x + 4.0f, hr.y + 5.0f, t->primary);
                r_pop_clip();

                // Value column with clipping
                r_push_clip(rect_make(hr.x + key_col_w + 8.0f, hr.y, hr.w - key_col_w - 12.0f, hr.h));
                font_draw_text(g_ui.font_mono, resp->headers[hi].value, 0, hr.x + key_col_w + 8.0f, hr.y + 5.0f, t->foreground);
                r_pop_clip();

                hy += 28.0f;
            }
            break;
        }
        case 3: { // Timeline Tab
            float ty = content_rect.y + 12.0f;
            font_draw_text(g_ui.font_bold, "Request Execution Timeline", 0, content_rect.x + 16.0f, ty, t->foreground);
            ty += 30.0f;

            double total_dur = (double)resp->latency_ms;
            if (total_dur <= 0) total_dur = 1.0;

            float name_col_w = 160.0f;
            float dur_col_w = 80.0f;

            for (size_t ti = 0; ti < resp->timeline_count; ti++) {
                TimelineEvent* ev = &resp->timeline[ti];
                Rect row_rect = rect_make(content_rect.x + 16.0f, ty, content_rect.w - 32.0f, 30.0f);

                if (rect_contains(row_rect, g_ui.mouse_x, g_ui.mouse_y)) {
                    r_draw_rect_rounded(row_rect, 4.0f, t->secondary_hover);
                }

                // Event Name with clipping
                r_push_clip(rect_make(row_rect.x, row_rect.y, name_col_w - 8.0f, row_rect.h));
                font_draw_text(g_ui.font_ui, ev->name, 0, row_rect.x + 4.0f, row_rect.y + 6.0f, t->foreground);
                r_pop_clip();

                // Timeline Bar
                float bar_x = row_rect.x + name_col_w;
                float bar_max_w = row_rect.w - name_col_w - dur_col_w - 16.0f;
                if (bar_max_w < 50.0f) bar_max_w = 50.0f;

                float ev_offset_w = (float)(ev->start_offset_ms / total_dur) * bar_max_w;
                float ev_w = (float)(ev->duration_ms / total_dur) * bar_max_w;
                if (ev_w < 4.0f) ev_w = 4.0f;

                // Background track
                r_draw_rect_rounded(rect_make(bar_x, row_rect.y + 9.0f, bar_max_w, 10.0f), 3.0f, t->secondary);
                // Active phase bar
                r_draw_rect_rounded(rect_make(bar_x + ev_offset_w, row_rect.y + 9.0f, ev_w, 10.0f), 3.0f, t->primary);

                // Duration text
                char dur_str[32];
                snprintf(dur_str, sizeof(dur_str), "%.1f ms", ev->duration_ms);
                font_draw_text(g_ui.font_mono, dur_str, 0, bar_x + bar_max_w + 12.0f, row_rect.y + 6.0f, t->text_muted);

                ty += 34.0f;
            }
            break;
        }
    }
}

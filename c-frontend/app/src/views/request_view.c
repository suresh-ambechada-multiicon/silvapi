#include "app/views.h"
#include "app/config.h"
#include "app/legacy_widgets.h"
#include "gfx/gfx.h"
#include "ui/layout.h"
#include "ui/components/tabs.h"
#include "input/input.h"

static void new_request_in_first_collection(void) {
    ApiRequest* new_req = api_request_create("New Request", METHOD_GET,
                                            "https://jsonplaceholder.typicode.com/posts/1");
    if (g_app.workspace && g_app.workspace->collections_count > 0) {
        Collection* col = g_app.workspace->collections[0];
        CollectionItem* ci = calloc(1, sizeof(CollectionItem));
        if (ci) {
            ci->type = ITEM_REQUEST;
            ci->request = new_req;
            CollectionItem** grown = realloc(col->items,
                                            sizeof(CollectionItem*) * (col->items_count + 1));
            if (grown) {
                col->items = grown;
                col->items[col->items_count++] = ci;
                storage_save_workspace(g_app.workspace);
            }
        }
    }
    app_open_request(new_req);
}

static const char* s_method_options[] = {
    "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"
};

static const char* s_body_type_options[] = {
    "No Body", "JSON", "Form Data", "Url Encoded", "Raw", "Binary File"
};

static const char* s_auth_type_options[] = {
    "None", "Bearer", "Basic", "API Key", "AWS Sig v4", "JWT", "OAuth 2.0"
};

// Everything below the document tab strip is still immediate-mode: the URL bar,
// sub-tabs, body editor, and key-value editors have not been ported. They are
// drawn inside a uil_custom block, which takes part in layout as an ordinary
// element -- so it cannot overlap the chrome above it -- while its interior is
// left alone. This is the incremental path the custom escape hatch exists for.
static void draw_request_body_legacy(Rect rect, void* user) {
    (void)user;
    Theme* t = theme_get_active();

    r_draw_rect(rect, t->background);

    // The tab strip now lives above this block, so the offsets that used to
    // skip past it are zero here.
    float tab_h = 0.0f;
    if (g_app.tab_count == 0 || g_app.active_tab_index < 0 || g_app.active_tab_index >= (int)g_app.tab_count) {
        // No active tab
        font_draw_text(g_ui.font_ui, "Select or create a request from the sidebar", 0, rect.x + 30.0f, rect.y + 80.0f, t->text_muted);
        return;
    }

    ApiRequest* req = g_app.open_requests[g_app.active_tab_index];
    if (!req) return;

    // 2. URL Bar & Method Dropdown & Send Button
    float url_bar_y = rect.y + tab_h + 10.0f;
    float url_bar_h = 32.0f;
    Rect url_bar_rect = rect_make(rect.x + 10.0f, url_bar_y, rect.w - 20.0f, url_bar_h);
    UILayout url_row = ui_layout_row(url_bar_rect, 6.0f, 0.0f, 0.0f);

    // Send / Cancel Button (anchored to right)
    Rect send_rect = ui_layout_item_right(&url_row, 85.0f, url_bar_h);

    if (g_app.is_sending) {
        if (ui_button_danger_icon(ICON_STOP, "Cancel", send_rect)) {
            app_cancel_active_request();
        }
    } else {
        if (ui_button_primary_icon(ICON_SEND, "Send", send_rect)) {
            app_send_active_request();
        }
    }

    // Method Dropdown (fixed on left)
    Rect method_rect = ui_layout_item_fixed(&url_row, 95.0f, url_bar_h);
    int cur_method = (int)req->method;
    if (ui_dropdown("req_method_sel", s_method_options, 7, &cur_method, method_rect)) {
        req->method = (HttpMethod)cur_method;
        g_app.tabs[g_app.active_tab_index].method = req->method;
        storage_save_workspace(g_app.workspace);
    }

    // URL Text Input with Lock / Globe Icon (fills entire remaining flex width)
    Rect url_rect = ui_layout_item_flex(&url_row, 1.0f, 80.0f, url_bar_h);

    if (ui_input_text_icon("req_url_input", ICON_LOCK, req->url, sizeof(req->url), "https://api.example.com/endpoint", url_rect)) {
        storage_save_workspace(g_app.workspace);
    }

    // 3. Request Sub-tabs (Body, Params, Headers, Auth, Info) with Lucide Icons
    float subtab_y = url_bar_y + url_bar_h + 10.0f;
    float subtab_h = 28.0f;

    char label_params[64], label_headers[64], label_body[64];
    snprintf(label_params, sizeof(label_params), "Params%s", req->params_count > 0 ? " *" : "");
    snprintf(label_headers, sizeof(label_headers), "Headers (%zu)", req->headers_count);
    snprintf(label_body, sizeof(label_body), "Body (%s)", body_type_to_string(req->body.body_type));

    const char* subtabs[] = {
        label_body,
        label_params,
        label_headers,
        "Auth",
        "Info"
    };

    const IconType subtab_icons[] = {
        ICON_FILE_CODE,
        ICON_TABLE,
        ICON_LAYERS,
        ICON_SHIELD,
        ICON_INFO
    };

    Rect subtab_bar = rect_make(rect.x + 10.0f, subtab_y, rect.w - 20.0f, subtab_h);
    g_app.request_subtab = ui_tab_bar_icons("req_subtabs", subtab_icons, subtabs, 5, g_app.request_subtab, subtab_bar);

    // 4. Sub-tab Content Area
    float content_y = subtab_y + subtab_h + 8.0f;
    float content_h = rect.y + rect.h - content_y - 8.0f;
    Rect content_rect = rect_make(rect.x + 10.0f, content_y, rect.w - 20.0f, content_h);

    switch (g_app.request_subtab) {
        case 0: { // Body Tab
            // Body Type Selector (Radio/Dropdown buttons) with clipping & layout
            Rect bth_rect = rect_make(content_rect.x, content_rect.y, content_rect.w, 24.0f);
            UILayout bth_row = ui_layout_row(bth_rect, 4.0f, 0.0f, 0.0f);
            r_push_clip(bth_rect);

            for (int b = 0; b < 6; b++) {
                float bw = font_measure_text(g_ui.font_ui, s_body_type_options[b], 0) + 16.0f;
                if (!ui_layout_can_fit(&bth_row, bw) && b > 0) break;
                Rect br = ui_layout_item_fixed(&bth_row, bw, 24.0f);
                bool is_active_type = ((int)req->body.body_type == b);
                if (ui_button_icon(ICON_NONE, s_body_type_options[b], br, is_active_type)) {
                    req->body.body_type = (BodyType)b;
                    storage_save_workspace(g_app.workspace);
                }
            }
            r_pop_clip();

            // Body Editor
            Rect editor_rect = rect_make(content_rect.x, content_rect.y + 30.0f, content_rect.w, content_rect.h - 30.0f);
            if (req->body.body_type == BODY_NO_BODY) {
                font_draw_text(g_ui.font_ui, "This request does not have a body", 0, editor_rect.x + 8.0f, editor_rect.y + 20.0f, t->text_muted);
            } else if (req->body.body_type == BODY_JSON || req->body.body_type == BODY_RAW) {
                ui_code_editor_legacy("req_body_editor", &req->body.content, &req->body.content_len, &req->body.content_cap,
                               true, (req->body.body_type == BODY_JSON), false, editor_rect, &g_app.request_body_scroll_y);
            } else if (req->body.body_type == BODY_URLENCODED) {
                if (ui_kv_editor_legacy("req_urlencoded", &req->body.urlencoded, &req->body.urlencoded_count, editor_rect, &g_app.request_params_scroll_y)) {
                    storage_save_workspace(g_app.workspace);
                }
            } else if (req->body.body_type == BODY_BINARY) {
                font_draw_text(g_ui.font_ui, "Binary File Path:", 0, editor_rect.x, editor_rect.y + 6.0f, t->foreground);
                Rect f_input = rect_make(editor_rect.x, editor_rect.y + 28.0f, editor_rect.w, 28.0f);
                ui_input_text("req_bin_file", req->body.binary_file_path, sizeof(req->body.binary_file_path), "/path/to/file", f_input);
            }
            break;
        }
        case 1: { // Params Tab
            if (ui_kv_editor_legacy("req_params", &req->params, &req->params_count, content_rect, &g_app.request_params_scroll_y)) {
                storage_save_workspace(g_app.workspace);
            }
            break;
        }
        case 2: { // Headers Tab
            if (ui_kv_editor_legacy("req_headers", &req->headers, &req->headers_count, content_rect, &g_app.request_headers_scroll_y)) {
                storage_save_workspace(g_app.workspace);
            }
            break;
        }
        case 3: { // Auth Tab
            float fh_ui = font_get_height(g_ui.font_ui);
            float cur_ay = content_rect.y;
            font_draw_text(g_ui.font_ui, "Type:", 0, content_rect.x, cur_ay + (26.0f - fh_ui) * 0.5f, t->text_muted);
            
            Rect auth_sel = rect_make(content_rect.x + 50.0f, cur_ay, 140.0f, 26.0f);
            int cur_auth = (int)req->auth.type;
            if (ui_dropdown("req_auth_sel", s_auth_type_options, 7, &cur_auth, auth_sel)) {
                req->auth.type = (AuthType)cur_auth;
                storage_save_workspace(g_app.workspace);
            }
            cur_ay += 36.0f;

            if (req->auth.type == AUTH_NONE) {
                font_draw_text(g_ui.font_ui, "This request does not use any authorization", 0, content_rect.x, cur_ay + 10.0f, t->text_muted);
            } else if (req->auth.type == AUTH_BEARER) {
                font_draw_text(g_ui.font_ui, "Token:", 0, content_rect.x, cur_ay + (28.0f - fh_ui) * 0.5f, t->foreground);
                Rect tr = rect_make(content_rect.x + 60.0f, cur_ay, content_rect.w - 60.0f, 28.0f);
                if (ui_input_text("auth_bearer_tok", req->auth.bearer_token, sizeof(req->auth.bearer_token), "{{token}}", tr)) {
                    storage_save_workspace(g_app.workspace);
                }
            } else if (req->auth.type == AUTH_BASIC) {
                font_draw_text(g_ui.font_ui, "Username:", 0, content_rect.x, cur_ay + (28.0f - fh_ui) * 0.5f, t->foreground);
                Rect ur = rect_make(content_rect.x + 80.0f, cur_ay, content_rect.w - 80.0f, 28.0f);
                ui_input_text("auth_basic_user", req->auth.basic_username, sizeof(req->auth.basic_username), "Username", ur);
                cur_ay += 34.0f;
                font_draw_text(g_ui.font_ui, "Password:", 0, content_rect.x, cur_ay + (28.0f - fh_ui) * 0.5f, t->foreground);
                Rect pr = rect_make(content_rect.x + 80.0f, cur_ay, content_rect.w - 80.0f, 28.0f);
                ui_input_text("auth_basic_pass", req->auth.basic_password, sizeof(req->auth.basic_password), "Password", pr);
            } else if (req->auth.type == AUTH_API_KEY) {
                font_draw_text(g_ui.font_ui, "Key:", 0, content_rect.x, cur_ay + (28.0f - fh_ui) * 0.5f, t->foreground);
                Rect kr = rect_make(content_rect.x + 50.0f, cur_ay, (content_rect.w - 60.0f) * 0.4f, 28.0f);
                ui_input_text("auth_api_k", req->auth.api_key_name, sizeof(req->auth.api_key_name), "X-API-Key", kr);
                
                font_draw_text(g_ui.font_ui, "Value:", 0, kr.x + kr.w + 10.0f, cur_ay + (28.0f - fh_ui) * 0.5f, t->foreground);
                Rect vr = rect_make(kr.x + kr.w + 60.0f, cur_ay, content_rect.w - (kr.x + kr.w + 60.0f - content_rect.x), 28.0f);
                ui_input_text("auth_api_v", req->auth.api_key_value, sizeof(req->auth.api_key_value), "Value", vr);
            }
            break;
        }
        case 4: { // Info Tab
            float fh_ui = font_get_height(g_ui.font_ui);
            font_draw_text(g_ui.font_ui, "Name:", 0, content_rect.x, content_rect.y + (28.0f - fh_ui) * 0.5f, t->foreground);
            Rect nr = rect_make(content_rect.x + 50.0f, content_rect.y, content_rect.w - 50.0f, 28.0f);
            if (ui_input_text("req_name_input", req->name, sizeof(req->name), "Request Name", nr)) {
                strncpy(g_app.tabs[g_app.active_tab_index].title, req->name, sizeof(g_app.tabs[g_app.active_tab_index].title) - 1);
                storage_save_workspace(g_app.workspace);
            }

            font_draw_text(g_ui.font_ui, "Description:", 0, content_rect.x, content_rect.y + 40.0f, t->foreground);
            Rect dr = rect_make(content_rect.x, content_rect.y + 64.0f, content_rect.w, content_rect.h - 64.0f);
            ui_input_text("req_desc_input", req->description, sizeof(req->description), "Add markdown description...", dr);
            break;
        }
    }
}


// ---------------------------------------------------------------------------
// Request panel.
//
// The document tab strip is a component now. The previous version measured each
// tab with font_measure_text + 65.0f, clamped it between 120 and 200, advanced a
// cur_tab_x cursor, and passed an app-level HttpMethod straight into the widget
// library -- which is what forced libs/ui to depend on the application's model.
// ---------------------------------------------------------------------------

void view_request_panel(Rect rect) {
    Theme* t = theme_get_active();

    // Tabs are rebuilt each frame from the open-request list. The array is
    // per-frame scratch because the component only reads it during the call.
    static UiTab tabs[MAX_OPEN_TABS];
    size_t n = 0;
    for (size_t i = 0; i < g_app.tab_count && n < MAX_OPEN_TABS; i++) {
        ApiRequest* req = g_app.open_requests[i];
        if (!req) continue;
        const char* method = http_method_to_string(req->method);
        tabs[n++] = (UiTab){
            .id = req->id,
            .label = req->name,
            .badge = method,
            .badge_color = theme_get_method_color(method),
            .dirty = g_app.tabs[i].is_dirty,
            .closable = true,
            .user = req,
        };
    }

    UI_PANEL(rect, .bg = t->background) {
        UI_COL(.w = UI_GROW, .h = UI_GROW, .gap = 0.0f) {

            if (n > 0) {
                UiTabsResult tr = ui_tabs(&(UiTabsProps){
                    .id = "req.tabs",
                    .tabs = tabs,
                    .tab_count = n,
                    .active = &g_app.active_tab_index,
                    .height = 32.0f,
                    .max_tab_width = 200.0f,
                    .show_add = true,
                    .show_overflow = true,
                    .reorderable = true,
                    .close_on_middle = true,
                });

                if (tr.add_clicked) new_request_in_first_collection();
                if (tr.closed >= 0) app_close_tab(tr.closed);

                if (tr.reorder_from >= 0 && tr.reorder_to >= 0) {
                    int a = tr.reorder_from, b = tr.reorder_to;
                    if (a < (int)g_app.tab_count && b < (int)g_app.tab_count) {
                        TabItem   ti = g_app.tabs[a];
                        ApiRequest* rq = g_app.open_requests[a];
                        // Shift the span between the two slots, then drop the
                        // dragged tab into its new index.
                        if (a < b) {
                            for (int k = a; k < b; k++) {
                                g_app.tabs[k] = g_app.tabs[k + 1];
                                g_app.open_requests[k] = g_app.open_requests[k + 1];
                            }
                        } else {
                            for (int k = a; k > b; k--) {
                                g_app.tabs[k] = g_app.tabs[k - 1];
                                g_app.open_requests[k] = g_app.open_requests[k - 1];
                            }
                        }
                        g_app.tabs[b] = ti;
                        g_app.open_requests[b] = rq;
                        if (g_app.active_tab_index == a) g_app.active_tab_index = b;
                    }
                }
            } else {
                UI_ROW(.w = UI_GROW, .h = UI_PX(32.0f), .pad_x = 8.0f,
                       .align_y = UI_CENTER, .bg = t->tabbar_bg) {
                    if (UI_BUTTON(.id = "req.newtab", .icon = ICON_PLUS,
                                  .label = "New Request", .icon_size = 12.0f,
                                  .h = UI_PX(22.0f), .radius = g_config.corner_radius,
                                  .fg = t->text_muted, .fg_hover = t->primary,
                                  .bg_hover = t->secondary_hover)) {
                        new_request_in_first_collection();
                    }
                }
            }

            UI_BOX(.w = UI_GROW, .h = UI_PX(1.0f), .bg = t->border) {}

            uil_custom(UI_GROW, UI_GROW, draw_request_body_legacy, NULL);
        }
    }
}

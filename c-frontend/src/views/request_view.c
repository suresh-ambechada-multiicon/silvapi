#include "views.h"
#include "gl_backend.h"

static const char* s_method_options[] = {
    "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"
};

static const char* s_body_type_options[] = {
    "No Body", "JSON", "Form Data", "Url Encoded", "Raw", "Binary File"
};

static const char* s_auth_type_options[] = {
    "None", "Bearer", "Basic", "API Key", "AWS Sig v4", "JWT", "OAuth 2.0"
};

void view_request_panel(Rect rect) {
    Theme* t = theme_get_active();

    r_draw_rect(rect, t->background);
    r_draw_line((Vec2){rect.x + rect.w, rect.y}, (Vec2){rect.x + rect.w, rect.y + rect.h}, 1.0f, t->border);

    // 1. Request Tab Strip
    float tab_h = 32.0f;
    Rect tab_strip_rect = rect_make(rect.x, rect.y, rect.w, tab_h);
    r_draw_rect(tab_strip_rect, t->tabbar_bg);
    r_draw_line((Vec2){rect.x, rect.y + tab_h}, (Vec2){rect.x + rect.w, rect.y + tab_h}, 1.0f, t->border);

    float cur_tab_x = rect.x;
    for (size_t i = 0; i < g_app.tab_count; i++) {
        ApiRequest* req = g_app.open_requests[i];
        if (!req) continue;

        float tw = font_measure_text(g_ui.font_ui, req->name, 0) + 65.0f;
        if (tw < 120.0f) tw = 120.0f;
        if (tw > 200.0f) tw = 200.0f;

        Rect tab_r = rect_make(cur_tab_x, rect.y, tw, tab_h);
        bool is_active = (g_app.active_tab_index == (int)i);
        bool close_clicked = false;

        if (ui_tab_item(req->name, req->method, is_active, g_app.tabs[i].is_dirty, true, &close_clicked, tab_r)) {
            g_app.active_tab_index = (int)i;
        }

        if (close_clicked) {
            app_close_tab((int)i);
            break;
        }

        cur_tab_x += tw;
    }

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

    // Method Dropdown
    float method_w = 95.0f;
    Rect method_rect = rect_make(rect.x + 10.0f, url_bar_y, method_w, url_bar_h);
    int cur_method = (int)req->method;
    if (ui_dropdown("req_method_sel", s_method_options, 7, &cur_method, method_rect)) {
        req->method = (HttpMethod)cur_method;
        g_app.tabs[g_app.active_tab_index].method = req->method;
        storage_save_workspace(g_app.workspace);
    }

    // Send / Cancel Button
    float send_w = 75.0f;
    Rect send_rect = rect_make(rect.x + rect.w - send_w - 10.0f, url_bar_y, send_w, url_bar_h);

    if (g_app.is_sending) {
        if (ui_button_danger("Cancel", send_rect)) {
            app_cancel_active_request();
        }
    } else {
        if (ui_button("Send", send_rect, true)) {
            app_send_active_request();
        }
    }

    // URL Text Input
    float url_x = method_rect.x + method_rect.w + 6.0f;
    float url_w = send_rect.x - url_x - 6.0f;
    Rect url_rect = rect_make(url_x, url_bar_y, url_w, url_bar_h);

    if (ui_input_text("req_url_input", req->url, sizeof(req->url), "https://api.example.com/endpoint", url_rect)) {
        storage_save_workspace(g_app.workspace);
    }

    // 3. Request Sub-tabs (Body, Params, Headers, Auth, Info)
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

    Rect subtab_bar = rect_make(rect.x + 10.0f, subtab_y, rect.w - 20.0f, subtab_h);
    g_app.request_subtab = ui_tab_bar("req_subtabs", subtabs, 5, g_app.request_subtab, subtab_bar);

    // 4. Sub-tab Content Area
    float content_y = subtab_y + subtab_h + 8.0f;
    float content_h = rect.y + rect.h - content_y - 8.0f;
    Rect content_rect = rect_make(rect.x + 10.0f, content_y, rect.w - 20.0f, content_h);

    switch (g_app.request_subtab) {
        case 0: { // Body Tab
            // Body Type Selector (Radio/Dropdown buttons)
            float bth_y = content_rect.y;
            float cur_bx = content_rect.x;
            for (int b = 0; b < 6; b++) {
                float bw = font_measure_text(g_ui.font_ui, s_body_type_options[b], 0) + 16.0f;
                Rect br = rect_make(cur_bx, bth_y, bw, 24.0f);
                bool is_active_type = ((int)req->body.body_type == b);
                if (ui_button_icon(ICON_NONE, s_body_type_options[b], br, is_active_type)) {
                    req->body.body_type = (BodyType)b;
                    storage_save_workspace(g_app.workspace);
                }
                cur_bx += bw + 6.0f;
            }

            // Body Editor
            Rect editor_rect = rect_make(content_rect.x, bth_y + 30.0f, content_rect.w, content_rect.h - 30.0f);
            if (req->body.body_type == BODY_NO_BODY) {
                font_draw_text(g_ui.font_ui, "This request does not have a body", 0, editor_rect.x + 8.0f, editor_rect.y + 20.0f, t->text_muted);
            } else if (req->body.body_type == BODY_JSON || req->body.body_type == BODY_RAW) {
                ui_code_editor("req_body_editor", &req->body.content, &req->body.content_len, &req->body.content_cap,
                               true, (req->body.body_type == BODY_JSON), false, editor_rect, &g_app.request_body_scroll_y);
            } else if (req->body.body_type == BODY_URLENCODED) {
                if (ui_kv_editor("req_urlencoded", &req->body.urlencoded, &req->body.urlencoded_count, editor_rect, &g_app.request_params_scroll_y)) {
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
            if (ui_kv_editor("req_params", &req->params, &req->params_count, content_rect, &g_app.request_params_scroll_y)) {
                storage_save_workspace(g_app.workspace);
            }
            break;
        }
        case 2: { // Headers Tab
            if (ui_kv_editor("req_headers", &req->headers, &req->headers_count, content_rect, &g_app.request_headers_scroll_y)) {
                storage_save_workspace(g_app.workspace);
            }
            break;
        }
        case 3: { // Auth Tab
            float cur_ay = content_rect.y;
            font_draw_text(g_ui.font_ui, "Type:", 0, content_rect.x, cur_ay + 6.0f, t->text_muted);
            
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
                font_draw_text(g_ui.font_ui, "Token:", 0, content_rect.x, cur_ay + 6.0f, t->foreground);
                Rect tr = rect_make(content_rect.x + 60.0f, cur_ay, content_rect.w - 60.0f, 28.0f);
                if (ui_input_text("auth_bearer_tok", req->auth.bearer_token, sizeof(req->auth.bearer_token), "{{token}}", tr)) {
                    storage_save_workspace(g_app.workspace);
                }
            } else if (req->auth.type == AUTH_BASIC) {
                font_draw_text(g_ui.font_ui, "Username:", 0, content_rect.x, cur_ay + 6.0f, t->foreground);
                Rect ur = rect_make(content_rect.x + 80.0f, cur_ay, content_rect.w - 80.0f, 28.0f);
                ui_input_text("auth_basic_user", req->auth.basic_username, sizeof(req->auth.basic_username), "Username", ur);
                cur_ay += 34.0f;
                font_draw_text(g_ui.font_ui, "Password:", 0, content_rect.x, cur_ay + 6.0f, t->foreground);
                Rect pr = rect_make(content_rect.x + 80.0f, cur_ay, content_rect.w - 80.0f, 28.0f);
                ui_input_text("auth_basic_pass", req->auth.basic_password, sizeof(req->auth.basic_password), "Password", pr);
            } else if (req->auth.type == AUTH_API_KEY) {
                font_draw_text(g_ui.font_ui, "Key:", 0, content_rect.x, cur_ay + 6.0f, t->foreground);
                Rect kr = rect_make(content_rect.x + 50.0f, cur_ay, (content_rect.w - 60.0f) * 0.4f, 28.0f);
                ui_input_text("auth_api_k", req->auth.api_key_name, sizeof(req->auth.api_key_name), "X-API-Key", kr);
                
                font_draw_text(g_ui.font_ui, "Value:", 0, kr.x + kr.w + 10.0f, cur_ay + 6.0f, t->foreground);
                Rect vr = rect_make(kr.x + kr.w + 60.0f, cur_ay, content_rect.w - (kr.x + kr.w + 60.0f - content_rect.x), 28.0f);
                ui_input_text("auth_api_v", req->auth.api_key_value, sizeof(req->auth.api_key_value), "Value", vr);
            }
            break;
        }
        case 4: { // Info Tab
            font_draw_text(g_ui.font_ui, "Name:", 0, content_rect.x, content_rect.y + 6.0f, t->foreground);
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

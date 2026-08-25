#include "base/base.h"
#include "gfx/gfx.h"
#include "ui/theme.h"
#include "text/font.h"
#include "app/model.h"
#include "app/storage.h"
#include "app/http_client.h"
#include "json/json_utils.h"
#include "ui/editbuf.h"
#include "ui/dock.h"
#include "ui/ui.h"
#include "icons/icon.h"
#include "ui/layout.h"
#include "ui/style.h"
#include "ui/components/kv_editor.h"
#include "app/views.h"
#include "app/config.h"
#include "input/input.h"
#include "input/input_glfw.h"
#include "GLFW/glfw3.h"

AppState g_app = {0};

// ---------------------------------------------------------------------------
// Dock view registration. libs/ui's dock takes (Rect, void*) callbacks and has
// no knowledge of what a panel contains, so each Silvapi view gets a thin
// adapter here. The Variables and History panels used to live inside the dock
// itself; they belong in the app.
// ---------------------------------------------------------------------------
static void panel_response(Rect r, void* u) { (void)u; view_response_panel(r); }

// Declarative, emitted from the Clay pass; see the VIEW_VARIABLES case there.
void view_variables_panel(Rect rect) {
    Theme* t = theme_get_active();
    if (!g_app.workspace) return;

    UI_PANEL(rect, .bg = t->background) {
        UI_COL(.w = UI_GROW, .h = UI_GROW, .pad = 12.0f, .gap = 8.0f) {
            UI_LABEL("Workspace Variables", .font = UI_FONT_BOLD, .color = t->foreground);

            static AppKvBinding bind;
            bind.items = &g_app.workspace->variables;
            bind.count = &g_app.workspace->variables_count;

            UiKvEditorResult kr = ui_kv_editor(&(UiKvEditorProps){
                .id = "panel.ws_vars",
                .row_count = g_app.workspace->variables_count,
                .row = app_kv_row,
                .user = &bind,
                .show_header = true,
                .allow_add = true,
                .allow_remove = true,
            });

            if (kr.add_clicked) {
                kv_array_add(&g_app.workspace->variables,
                             &g_app.workspace->variables_count, "", "", true);
            }
            if (kr.removed >= 0) {
                Workspace* ws = g_app.workspace;
                for (size_t k = (size_t)kr.removed; k + 1 < ws->variables_count; k++) {
                    ws->variables[k] = ws->variables[k + 1];
                }
                if (ws->variables_count > 0) ws->variables_count--;
            }
            if (kr.changed || kr.removed >= 0 || kr.toggled >= 0) {
                storage_save_workspace(g_app.workspace);
            }
        }
    }
}

static void panel_history(Rect r, void* u) {
    (void)u;
    Theme* t = theme_get_active();
    r_draw_rect(r, t->background);
    font_draw_text(g_ui.font_bold, "Execution History", 0, r.x + 16.0f, r.y + 16.0f, t->foreground);
}

// The single adapter between the app model and the kv-editor component. The
// component writes straight into these buffers, so nothing is copied and there
// is no second copy of the value being typed.
void app_kv_row(size_t index, struct UiKvRow* out, void* user) {
    AppKvBinding* b = (AppKvBinding*)user;
    if (!b || !b->items || !*b->items || !out) return;
    if (index >= *b->count) return;

    KeyValue* kv = &(*b->items)[index];
    UiKvRow* r = (UiKvRow*)out;
    r->id = kv->id;
    r->key = kv->key;
    r->key_cap = sizeof(kv->key);
    r->value = kv->value;
    r->value_cap = sizeof(kv->value);
    r->description = kv->description;
    r->description_cap = sizeof(kv->description);
    r->enabled = &kv->enabled;
}

void app_register_dock_views(void) {
    // Declarative: emitted inside the frame's Clay layout, not by the dock.
    dock_register_view_declarative(VIEW_SIDEBAR, "Collections", NULL);
    dock_register_view_declarative(VIEW_REQUEST, "Request", NULL);
    dock_register_view(VIEW_RESPONSE,  "Response",    panel_response,  NULL);
    dock_register_view_declarative(VIEW_VARIABLES, "Variables", NULL);
    dock_register_view(VIEW_HISTORY,   "History",     panel_history,   NULL);
}


static void on_http_response_received(const HttpResponse* response, void* user_data) {
    (void)user_data;
    g_app.is_sending = false;

    http_response_free_contents(&g_app.active_response);
    g_app.active_response = (HttpResponse){0};
    g_app.active_response.status_code = response->status_code;
    g_app.active_response.latency_ms = response->latency_ms;
    g_app.active_response.body_size = response->body_size;
    strncpy(g_app.active_response.status_text, response->status_text, sizeof(g_app.active_response.status_text) - 1);
    strncpy(g_app.active_response.timestamp, response->timestamp, sizeof(g_app.active_response.timestamp) - 1);

    if (response->body) {
        // Pretty-format JSON if valid
        char* formatted = json_format_string(response->body);
        g_app.active_response.body = formatted ? formatted : strdup(response->body);
    }
    if (response->error) {
        g_app.active_response.error = strdup(response->error);
    }

    // Clone headers
    if (response->headers_count > 0) {
        g_app.active_response.headers = malloc(sizeof(KeyValue) * response->headers_count);
        g_app.active_response.headers_count = response->headers_count;
        for (size_t i = 0; i < response->headers_count; i++) {
            strncpy(g_app.active_response.headers[i].key, response->headers[i].key, sizeof(g_app.active_response.headers[i].key) - 1);
            strncpy(g_app.active_response.headers[i].value, response->headers[i].value, sizeof(g_app.active_response.headers[i].value) - 1);
            g_app.active_response.headers[i].enabled = true;
        }
    }

    // Copy timeline
    g_app.active_response.timeline_count = response->timeline_count;
    for (size_t i = 0; i < response->timeline_count; i++) {
        g_app.active_response.timeline[i] = response->timeline[i];
    }

    // Invalidate cached JSON Table
    if (g_app.active_json_table) {
        json_table_free(g_app.active_json_table);
        g_app.active_json_table = NULL;
    }

    // Save to response history
    storage_add_response_history(g_app.last_sent_request_id, response);

    // Update last status on request
    if (g_app.tab_count > 0 && g_app.active_tab_index >= 0) {
        g_app.open_requests[g_app.active_tab_index]->last_status = response->status_code;
    }
}

void app_show_toast(const char* message) {
    if (message) {
        strncpy(g_app.toast_message, message, sizeof(g_app.toast_message) - 1);
        g_app.toast_timer = 2.5f;
    }
}

void app_open_request(ApiRequest* req) {
    if (!req) return;

    // Check if already open
    for (size_t i = 0; i < g_app.tab_count; i++) {
        if (strcmp(g_app.open_requests[i]->id, req->id) == 0) {
            g_app.active_tab_index = (int)i;
            return;
        }
    }

    if (g_app.tab_count < MAX_OPEN_TABS) {
        int idx = (int)g_app.tab_count;
        g_app.open_requests[idx] = req;
        strncpy(g_app.tabs[idx].request_id, req->id, sizeof(g_app.tabs[idx].request_id) - 1);
        strncpy(g_app.tabs[idx].title, req->name, sizeof(g_app.tabs[idx].title) - 1);
        g_app.tabs[idx].method = req->method;
        g_app.tabs[idx].is_dirty = false;
        g_app.tab_count++;
        g_app.active_tab_index = idx;
    }
}

void app_close_tab(int tab_idx) {
    if (tab_idx < 0 || tab_idx >= (int)g_app.tab_count) return;

    for (size_t i = tab_idx; i + 1 < g_app.tab_count; i++) {
        g_app.tabs[i] = g_app.tabs[i + 1];
        g_app.open_requests[i] = g_app.open_requests[i + 1];
    }
    g_app.tab_count--;

    if (g_app.active_tab_index >= (int)g_app.tab_count) {
        g_app.active_tab_index = (int)g_app.tab_count - 1;
    }
}

void app_send_active_request(void) {
    if (g_app.tab_count == 0 || g_app.active_tab_index < 0 || g_app.active_tab_index >= (int)g_app.tab_count) {
        return;
    }

    ApiRequest* req = g_app.open_requests[g_app.active_tab_index];
    if (!req || g_app.is_sending) return;

    g_app.is_sending = true;
    strncpy(g_app.last_sent_request_id, req->id, sizeof(g_app.last_sent_request_id) - 1);

    http_client_send(req, g_app.workspace, NULL, NULL, on_http_response_received, NULL);
}

void app_cancel_active_request(void) {
    if (g_app.is_sending && g_app.last_sent_request_id[0]) {
        http_client_cancel(g_app.last_sent_request_id);
        g_app.is_sending = false;
    }
}

static void open_all_seed_requests(CollectionItem* item) {
    if (!item) return;
    if (item->type == ITEM_REQUEST && item->request) {
        app_open_request(item->request);
    } else if (item->type == ITEM_FOLDER && item->folder) {
        for (size_t i = 0; i < item->folder->items_count; i++) {
            open_all_seed_requests(item->folder->items[i]);
        }
    }
}

void app_state_init(void) {
    g_app.sidebar_width = 240.0f;
    g_app.request_split_ratio = 0.5f;
    g_app.table_detail_height = 140.0f;
    g_app.request_subtab = 0;
    g_app.response_subtab = 0;

    // Load Workspace from SQLite
    g_app.workspace = storage_load_workspace();

    // Open initial requests into tab strip matching screenshot 1
    if (g_app.workspace && g_app.workspace->collections_count > 0) {
        Collection* col = g_app.workspace->collections[0];
        for (size_t i = 0; i < col->items_count; i++) {
            open_all_seed_requests(col->items[i]);
        }
    }

    if (g_app.tab_count > 0) {
        g_app.active_tab_index = 0;
        // Populate initial response snapshot so panel is active immediately
        g_app.active_response.status_code = 200;
        snprintf(g_app.active_response.status_text, sizeof(g_app.active_response.status_text), "OK");
        g_app.active_response.latency_ms = 294;
        const char* sample_json = "{\n  \"userId\": 1,\n  \"id\": 1,\n  \"title\": \"sunt aut facere repellat provident occaecati excepturi optio reprehenderit\",\n  \"body\": \"quia et suscipit\\nsuscipit recusandae consequuntur expedita et cum\\nreprehenderit molestiae ut ut quas totam\\nnostrum rerum est autem sunt rem eveniet architecto\"\n}";
        g_app.active_response.body = strdup(sample_json);
        g_app.active_response.body_size = strlen(sample_json);
        snprintf(g_app.active_response.timestamp, sizeof(g_app.active_response.timestamp), "2026-08-24 15:30:00");
    }

    // Load active theme from settings
    char* saved_theme = storage_load_setting("theme_preset");
    if (saved_theme) {
        theme_set_preset((ThemePreset)atoi(saved_theme));
        free(saved_theme);
    }
}

void app_state_shutdown(void) {
    if (g_app.workspace) {
        storage_save_workspace(g_app.workspace);
        workspace_free(g_app.workspace);
    }
    if (g_app.active_json_table) {
        json_table_free(g_app.active_json_table);
    }
    http_response_free_contents(&g_app.active_response);
}

// ---------------------------------------------------------------------------
// Input actions.
//
// Shortcuts used to live inside the GLFW key callback, where they could not be
// listed, rebound, or tested. They are now named bindings owned by libs/input:
// each is "when this chord fires, call this function", and the whole set can be
// enumerated (see app_shortcut_label) to build a shortcut help panel.
// ---------------------------------------------------------------------------

static void act_command_palette(void* u) {
    (void)u;
    g_app.show_command_palette = !g_app.show_command_palette;
    g_app.command_palette_query[0] = '\0';
    g_app.command_palette_sel_idx = 0;
}

static void act_send_request(void* u)   { (void)u; app_send_active_request(); }
static void act_toggle_sidebar(void* u) { (void)u; layout_toggle_sidebar(); }
static void act_close_tab(void* u)      { (void)u; app_close_tab(g_app.active_tab_index); }

static void act_save_workspace(void* u) {
    (void)u;
    storage_save_workspace(g_app.workspace);
    app_show_toast("Workspace saved");
}

static void act_open_settings(void* u) {
    (void)u;
    g_app.show_settings_modal = !g_app.show_settings_modal;
}

static void act_reset_layout(void* u) {
    (void)u;
    layout_reset_default((float)g_ui.window_width, (float)g_ui.window_height);
    app_show_toast("Layout reset to default");
}

static void app_register_shortcuts(void) {
    input_bind_key("command_palette", INPUT_KEY_P,     INPUT_MOD_CTRL, act_command_palette, NULL);
    input_bind_key("send_request",    INPUT_KEY_ENTER, INPUT_MOD_CTRL, act_send_request,    NULL);
    input_bind_key("toggle_sidebar",  INPUT_KEY_B,     INPUT_MOD_CTRL, act_toggle_sidebar,  NULL);
    input_bind_key("close_tab",       INPUT_KEY_W,     INPUT_MOD_CTRL, act_close_tab,       NULL);
    input_bind_key("save_workspace",  INPUT_KEY_S,     INPUT_MOD_CTRL, act_save_workspace,  NULL);
    input_bind_key("open_settings",   INPUT_KEY_COMMA, INPUT_MOD_CTRL, act_open_settings,   NULL);
    input_bind_key("reset_layout",    INPUT_KEY_R,     INPUT_MOD_CTRL | INPUT_MOD_SHIFT,
                   act_reset_layout, NULL);
}

// Bridges the input library's state into the legacy immediate-mode UIContext,
// so existing widgets keep working while views are ported.
static void app_sync_ui_input(void) {
    Vec2 m = input_mouse();
    g_ui.mouse_x = m.x;
    g_ui.mouse_y = m.y;

    for (int b = 0; b < 3; b++) {
        g_ui.mouse_down[b]     = input_mouse_down(b);
        g_ui.mouse_clicked[b]  = input_mouse_pressed(b);
        g_ui.mouse_released[b] = input_mouse_released(b);
    }
    g_ui.mouse_click_count = input_click_count();

    Vec2 sc = input_scroll();
    g_ui.scroll_x = sc.x;
    g_ui.scroll_y = sc.y;

    uint32_t mods = input_mods();
    g_ui.ctrl_down  = (mods & INPUT_MOD_CTRL)  != 0;
    g_ui.shift_down = (mods & INPUT_MOD_SHIFT) != 0;
    g_ui.alt_down   = (mods & INPUT_MOD_ALT)   != 0;

    // The legacy widgets read a single key code per frame.
    g_ui.key_pressed = 0;
    for (size_t i = 0; i < input_event_count(); i++) {
        const InputEvent* ev = input_event_at(i);
        if (ev && (ev->kind == INPUT_EV_KEY_DOWN || ev->kind == INPUT_EV_KEY_REPEAT)) {
            g_ui.key_pressed = ev->key;
            break;
        }
    }

    const char* typed = input_text();
    size_t n = strlen(typed);
    if (n >= sizeof(g_ui.char_input)) n = sizeof(g_ui.char_input) - 1;
    memcpy(g_ui.char_input, typed, n);
    g_ui.char_input[n] = '\0';
    g_ui.char_input_len = n;
}

int main(int argc, char** argv) {
    (void)argc;
    (void)argv;

    if (!glfwInit()) {
        fprintf(stderr, "Failed to initialize GLFW\n");
        return 1;
    }

    glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, 3);
    glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, 3);
    glfwWindowHint(GLFW_OPENGL_PROFILE, GLFW_OPENGL_CORE_PROFILE);
    glfwWindowHint(GLFW_SAMPLES, 4);

    int win_w = 1280;
    int win_h = 800;
    // Fixed size for reproducible captures: a tiling compositor otherwise hands
    // the window a different size every launch, which makes a trace and a
    // screenshot from two runs incomparable.
    if (getenv("SILVAPI_WINDOW")) {
        int w = 0, h = 0;
        if (sscanf(getenv("SILVAPI_WINDOW"), "%dx%d", &w, &h) == 2 && w > 0 && h > 0) {
            win_w = w;
            win_h = h;
            glfwWindowHint(GLFW_RESIZABLE, GLFW_FALSE);
        }
    }
    // SILVAPI_FULLSCREEN gives a predictable large surface for capture; tiling
    // compositors ignore both the requested size and GLFW_RESIZABLE.
    GLFWmonitor* mon = NULL;
    if (getenv("SILVAPI_FULLSCREEN")) {
        mon = glfwGetPrimaryMonitor();
        const GLFWvidmode* vm = mon ? glfwGetVideoMode(mon) : NULL;
        if (vm) {
            win_w = vm->width;
            win_h = vm->height;
        }
    }

    GLFWwindow* window = glfwCreateWindow(win_w, win_h, "Silvapi - API Client", mon, NULL);
    if (!window) {
        fprintf(stderr, "Failed to create GLFW window\n");
        glfwTerminate();
        return 1;
    }

    glfwMakeContextCurrent(window);
    // VSync on by default; off when benchmarking so frame cost is measurable.
    glfwSwapInterval(getenv("SILVAPI_NO_VSYNC") ? 0 : 1);

    // Callbacks
    input_init();
    input_glfw_attach(window);
    app_register_shortcuts();

    int fb_w, fb_h;
    glfwGetFramebufferSize(window, &fb_w, &fb_h);
    {
        int lw, lh; glfwGetWindowSize(window, &lw, &lh);
        float xs = 1.0f, ys = 1.0f;
        glfwGetWindowContentScale(window, &xs, &ys);
        fprintf(stderr, "[DPI] window=%dx%d framebuffer=%dx%d fb_scale=%.3f content_scale=%.3f\n",
                lw, lh, fb_w, fb_h, lw ? (float)fb_w/(float)lw : 0.0f, xs);
    }

    // Configuration first: it decides font sizes, DPI, and render tunables, so
    // it must land before any subsystem builds a cache from those values.
    if (!app_config_load(&g_config)) {
        fprintf(stderr, "[Config] no config file found, using defaults "
                        "(write one with SILVAPI_WRITE_CONFIG=path)\n");
    } else {
        fprintf(stderr, "[Config] loaded %s\n", g_config.source_path);
    }

    // dpi_scale <= 0 means follow the window. Prefer the framebuffer ratio,
    // which is what the geometry pipeline actually uses; fall back to the
    // monitor content scale when they disagree (some compositors report only
    // one of the two).
    if (g_config.dpi_scale <= 0.0f) {
        float fb_ratio = win_w > 0 ? (float)fb_w / (float)win_w : 1.0f;
        float cs_x = 1.0f, cs_y = 1.0f;
        glfwGetWindowContentScale(window, &cs_x, &cs_y);
        g_config.dpi_scale = fb_ratio > 1.01f ? fb_ratio : cs_x;
        if (g_config.dpi_scale <= 0.0f) g_config.dpi_scale = 1.0f;
    }
    app_config_apply(&g_config);

    // Style tokens come from the theme; components reference token names only.
    ui_style_set_spacing_unit(4.0f);
    ui_style_set_radius_scale(2.0f, g_config.corner_radius, g_config.corner_radius * 2.0f);
    ui_style_set_warn_unknown(getenv("SILVAPI_STYLE_WARN") != NULL);

    if (getenv("SILVAPI_WRITE_CONFIG")) {
        const char* out = getenv("SILVAPI_WRITE_CONFIG");
        fprintf(stderr, "[Config] %s %s\n",
                app_config_write(&g_config, out) ? "wrote" : "failed to write", out);
    }

    // Initialize systems
    storage_init();
    http_client_init();
    gl_backend_init(win_w, win_h, fb_w, fb_h);
    ui_init(win_w, win_h, (UiFontSpec){
        .size_ui = g_config.font_size_ui,
        .size_mono = g_config.font_size_mono,
        .size_bold = g_config.font_size_bold,
        .family_ui = g_config.font_family_ui,
        .family_mono = g_config.font_family_mono,
    });

    // Tokens are published by theme_init()/theme_set_preset(), which run inside
    // ui_init(); nothing to load here.
    if (getenv("SILVAPI_STYLE_WARN")) {
        Color c;
        fprintf(stderr, "[Style] fg=%d muted=%d surface=%d accent=%d primary=%d\n",
                ui_style_get_token("fg", &c), ui_style_get_token("muted", &c),
                ui_style_get_token("surface", &c), ui_style_get_token("accent", &c),
                ui_style_get_token("primary", &c));
        const UiStyle* rs = ui_style("row w-full items-center text-fg");
        Color f = ui_style_fg(rs, UI_STATE_BASE);
        fprintf(stderr, "[Style] text-fg resolves to (%.2f,%.2f,%.2f,a=%.2f)\n",
                f.r, f.g, f.b, f.a);
    }
    // Declarative layout engine. Font ids follow UiFontId ordering.
    {
        Font* layout_fonts[3] = { g_ui.font_ui, g_ui.font_mono, g_ui.font_bold };
        uil_layout_init(layout_fonts, 3);
    }
    app_register_dock_views();
    layout_init((float)win_w, (float)win_h, VIEW_SIDEBAR, VIEW_REQUEST, VIEW_RESPONSE);
    app_state_init();

    uil_set_overlap_check(getenv("SILVAPI_CHECK_OVERLAP") != NULL);
    uil_set_debug_text_boxes(getenv("SILVAPI_TEXT_BOXES") != NULL);
    uil_set_trace_buttons(getenv("SILVAPI_TRACE_BUTTONS") != NULL);

    // Test hook: open a dialog on startup so it can be captured and validated.
    // Swap a dock panel's active view, so a declarative panel can be captured.
    if (getenv("SILVAPI_SHOW_VIEW")) {
        const char* which = getenv("SILVAPI_SHOW_VIEW");
        ViewKind v = strcmp(which, "variables") == 0 ? VIEW_VARIABLES
                   : strcmp(which, "history")   == 0 ? VIEW_HISTORY
                   : strcmp(which, "request")   == 0 ? VIEW_REQUEST
                                                     : DOCK_VIEW_NONE;
        if (v != DOCK_VIEW_NONE) layout_set_active_view(g_layout.root, v);
    }

    if (getenv("SILVAPI_OPEN_DIALOG")) {
        const char* which = getenv("SILVAPI_OPEN_DIALOG");
        if      (strcmp(which, "settings")  == 0) g_app.show_settings_modal  = true;
        else if (strcmp(which, "variables") == 0) g_app.show_variables_modal = true;
        else if (strcmp(which, "palette")   == 0) g_app.show_command_palette = true;
        else if (strcmp(which, "import")    == 0) g_app.show_import_modal    = true;
    }

    // Test hook: run a fixed number of frames then exit cleanly, so the render
    // loop can be benchmarked and instrumented without a human closing it.
    long max_frames = 0;
    if (getenv("SILVAPI_MAX_FRAMES")) max_frames = atol(getenv("SILVAPI_MAX_FRAMES"));
    long frame_no = 0;

    double last_time = glfwGetTime();
    double start_time = last_time;

    while (!glfwWindowShouldClose(window)) {
        if (max_frames > 0 && ++frame_no > max_frames) break;
        double current_time = glfwGetTime();
        float dt = (float)(current_time - last_time);
        last_time = current_time;

        int logical_w, logical_h;
        glfwGetWindowSize(window, &logical_w, &logical_h);
        glfwGetFramebufferSize(window, &fb_w, &fb_h);

        g_ui.window_width = logical_w;
        g_ui.window_height = logical_h;

        gl_backend_resize(logical_w, logical_h, fb_w, fb_h);
        // Follow DPI changes (moving between monitors, fractional scale change).
        {
            float ratio = logical_w > 0 ? (float)fb_w / (float)logical_w : 1.0f;
            if (ratio > 0.0f && fabsf(ratio - text_get_config().dpi_scale) > 0.01f) {
                g_config.dpi_scale = ratio;
                app_config_apply(&g_config);
            }
        }

        if (getenv("SILVAPI_DPI_TRACE") && frame_no == 2) {
            float xs = 1.0f, ys = 1.0f;
            glfwGetWindowContentScale(window, &xs, &ys);
            fprintf(stderr, "[DPI] logical=%dx%d fb=%dx%d fb_scale=%.4f content_scale=%.4fx%.4f\n",
                    logical_w, logical_h, fb_w, fb_h,
                    logical_w ? (float)fb_w / (float)logical_w : 0.0f, xs, ys);
        }
        // Input first: begin_frame clears last frame's edges, then poll pumps
        // the platform callbacks into the event list.
        input_begin_frame(current_time);
        glfwPollEvents();

        // Click a control by ID rather than by coordinate. Window size varies
        // between launches here, so a hardcoded x,y aims at whatever the
        // previous run's layout happened to be.
        //   SILVAPI_CLICK_ID="code.wrap" [SILVAPI_CLICK_INDEX=n]
        if (getenv("SILVAPI_CLICK_ID")) {
            long at = getenv("SILVAPI_CLICK_FRAME") ? atol(getenv("SILVAPI_CLICK_FRAME")) : 10;
            int32_t idx = getenv("SILVAPI_CLICK_INDEX")
                            ? (int32_t)atol(getenv("SILVAPI_CLICK_INDEX")) : 0;
            Rect tr = uil_element_rect(getenv("SILVAPI_CLICK_ID"), idx);
            if (tr.w > 0.0f) {
                Vec2 c = { tr.x + tr.w * 0.5f, tr.y + tr.h * 0.5f };
                if (frame_no == at - 1 || frame_no == at) {
                    InputEvent mv = { .kind = INPUT_EV_MOUSE_MOVE, .pos = c };
                    input_push(&mv);
                }
                if (frame_no == at) {
                    InputEvent dn = { .kind = INPUT_EV_MOUSE_DOWN, .pos = c,
                                      .button = INPUT_MOUSE_LEFT, .time = current_time };
                    input_push(&dn);
                } else if (frame_no == at + 1) {
                    InputEvent up = { .kind = INPUT_EV_MOUSE_UP, .pos = c,
                                      .button = INPUT_MOUSE_LEFT, .time = current_time };
                    input_push(&up);
                }
            } else if (frame_no == at) {
                fprintf(stderr, "[Click] id '%s'[%d] not found in last frame's layout\n",
                        getenv("SILVAPI_CLICK_ID"), idx);
            }
        }

        // Synthetic click injection, so interaction can be tested without a
        // human: SILVAPI_CLICK="x,y" SILVAPI_CLICK_FRAME=n
        if (getenv("SILVAPI_CLICK")) {
            long click_frame = getenv("SILVAPI_CLICK_FRAME")
                                 ? atol(getenv("SILVAPI_CLICK_FRAME")) : 10;
            float cx = 0.0f, cy = 0.0f;
            if (sscanf(getenv("SILVAPI_CLICK"), "%f,%f", &cx, &cy) == 2) {
                // Move on the frame before, so hover state is established from
                // the previous layout the way a real pointer would.
                if (frame_no == click_frame - 1) {
                    InputEvent mv = { .kind = INPUT_EV_MOUSE_MOVE, .pos = { cx, cy } };
                    input_push(&mv);
                } else if (frame_no == click_frame) {
                    InputEvent mv = { .kind = INPUT_EV_MOUSE_MOVE, .pos = { cx, cy } };
                    input_push(&mv);
                    InputEvent dn = { .kind = INPUT_EV_MOUSE_DOWN, .pos = { cx, cy },
                                      .button = INPUT_MOUSE_LEFT, .time = current_time };
                    input_push(&dn);
                } else if (frame_no == click_frame + 1) {
                    InputEvent up = { .kind = INPUT_EV_MOUSE_UP, .pos = { cx, cy },
                                      .button = INPUT_MOUSE_LEFT, .time = current_time };
                    input_push(&up);
                }
            }
        }
        // Synthetic typing and key presses, so text inputs and keyboard
        // navigation can be tested without a human at the keyboard.
        //   SILVAPI_TYPE="expand"          types into the focused input
        //   SILVAPI_KEYS="down,down,enter" presses keys, one per frame
        if (getenv("SILVAPI_TYPE")) {
            long at = getenv("SILVAPI_TYPE_FRAME") ? atol(getenv("SILVAPI_TYPE_FRAME")) : 12;
            const char* text = getenv("SILVAPI_TYPE");
            if (frame_no == at) {
                for (const char* c = text; *c; c++) {
                    InputEvent ev = { .kind = INPUT_EV_TEXT, .codepoint = (uint32_t)*c };
                    input_push(&ev);
                }
            }
        }
        // Wheel injection, for verifying that a culled list scrolls correctly.
        //   SILVAPI_SCROLL="-40"  total wheel notches, spread one per frame
        if (getenv("SILVAPI_SCROLL")) {
            long at = getenv("SILVAPI_SCROLL_FRAME") ? atol(getenv("SILVAPI_SCROLL_FRAME")) : 12;
            long total = atol(getenv("SILVAPI_SCROLL"));
            long steps = total < 0 ? -total : total;
            float dir = total < 0 ? -1.0f : 1.0f;
            if (frame_no >= at && frame_no < at + steps) {
                InputEvent mv = { .kind = INPUT_EV_MOUSE_MOVE,
                                  .pos = { 200.0f, 400.0f } };
                input_push(&mv);
                InputEvent sc = { .kind = INPUT_EV_SCROLL, .scroll = { 0.0f, dir } };
                input_push(&sc);
            }
        }

        if (getenv("SILVAPI_KEYS")) {
            long at = getenv("SILVAPI_KEYS_FRAME") ? atol(getenv("SILVAPI_KEYS_FRAME")) : 20;
            static const struct { const char* name; int key; } KEYMAP[] = {
                {"down", INPUT_KEY_DOWN}, {"up", INPUT_KEY_UP},
                {"enter", INPUT_KEY_ENTER}, {"escape", INPUT_KEY_ESCAPE},
                {"tab", INPUT_KEY_TAB}, {"backspace", INPUT_KEY_BACKSPACE},
            };
            // One key per frame starting at `at`, so each is a distinct edge.
            long slot = frame_no - at;
            if (slot >= 0) {
                const char* spec = getenv("SILVAPI_KEYS");
                long i = 0;
                const char* p2 = spec;
                while (*p2 && i < slot) { if (*p2 == ',') i++; p2++; }
                if (i == slot && *p2) {
                    char name[24] = {0};
                    size_t k = 0;
                    while (p2[k] && p2[k] != ',' && k < sizeof(name) - 1) { name[k] = p2[k]; k++; }
                    for (size_t m = 0; m < sizeof(KEYMAP) / sizeof(KEYMAP[0]); m++) {
                        if (strcmp(name, KEYMAP[m].name) != 0) continue;
                        InputEvent dn = { .kind = INPUT_EV_KEY_DOWN, .key = KEYMAP[m].key,
                                          .time = current_time };
                        input_push(&dn);
                        break;
                    }
                }
            }
        }

        app_sync_ui_input();
        input_dispatch();

        gl_backend_begin_frame();
        ui_begin_frame(dt);

        Theme* t = theme_get_active();
        r_draw_rect(rect_make(0, 0, (float)logical_w, (float)logical_h), t->background);

        float topbar_h = g_config.topbar_height;
        Rect  window_rect = rect_make(0, 0, (float)logical_w, (float)logical_h);
        Rect  dock_rect   = rect_make(0, topbar_h, (float)logical_w,
                                      (float)logical_h - topbar_h);

        // The docked panels are still immediate-mode, so they paint before the
        // declarative pass. They occupy the region below the top bar and never
        // overlap it, so paint order between the two does not matter.
        bool demo_mode = getenv("SILVAPI_DEMO") != NULL;
        if (!demo_mode) {
            layout_update_bounds(g_layout.root, dock_rect, dt);
            layout_render_and_handle(g_layout.root);
        }

        // ONE Clay layout per frame. Clay keeps a single pointer-over set,
        // rebuilt from the most recent Clay_EndLayout, so a second layout in the
        // same frame wipes the first one's hit-test data and its buttons stop
        // responding to clicks entirely.
        uil_layout_begin(window_rect, (UiLayoutInput){
            .mouse = input_mouse(),
            .mouse_down    = { input_mouse_down(0),    input_mouse_down(1),    input_mouse_down(2) },
            .mouse_clicked = { input_mouse_pressed(0), input_mouse_pressed(1), input_mouse_pressed(2) },
            .scroll_delta  = input_scroll(),
            .dt = dt,
        });

        view_top_bar(rect_make(0, 0, (float)logical_w, topbar_h));
        UI_SPACER(UI_GROW);

        const char* demo = getenv("SILVAPI_DEMO");
        if (demo && strcmp(demo, "table") == 0) {
            view_demo_gallery(dock_rect);
        } else if (demo && strcmp(demo, "menu") == 0) {
            view_demo_menus(dock_rect);
        } else if (demo && strcmp(demo, "tree") == 0) {
            view_demo_tree(dock_rect);
        } else if (demo && strcmp(demo, "tabs") == 0) {
            view_demo_tabs(dock_rect);
        } else if (demo && strcmp(demo, "code") == 0) {
            view_demo_code(dock_rect);
        } else if (demo && strcmp(demo, "controls") == 0) {
            view_demo_controls(dock_rect);
        } else if (demo && strcmp(demo, "options") == 0) {
            view_demo_options(dock_rect);
        }

        // Docked panels that emit declarative content, drawn at the rects the
        // dock solved for them this frame.
        {
            DockViewRect panels[16];
            size_t n = dock_visible_views(panels, 16);
            for (size_t i = 0; i < n; i++) {
                switch (panels[i].id) {
                    case VIEW_SIDEBAR:  view_sidebar(panels[i].content_rect);  break;
                    case VIEW_REQUEST:  view_request_panel(panels[i].content_rect); break;
                    case VIEW_VARIABLES: view_variables_panel(panels[i].content_rect); break;
                    default: break;
                }
            }
        }

        view_dialogs();

        uil_layout_end();

        if (getenv("SILVAPI_CLICK") && frame_no == max_frames) {
            fprintf(stderr, "[Click] modals: import=%d variables=%d settings=%d palette=%d "
                            "sidebar_collapsed=%d\n",
                    g_app.show_import_modal, g_app.show_variables_modal,
                    g_app.show_settings_modal, g_app.show_command_palette,
                    g_layout.sidebar_collapsed);
        }

        if (getenv("SILVAPI_LAYOUT_TRACE") && r_clip_depth() != 0) {
            fprintf(stderr, "[Gfx] clip stack unbalanced at end of frame: depth=%d\n",
                    r_clip_depth());
        }

        ui_end_frame();
        gl_backend_end_frame();

        // Native-resolution capture for render-quality inspection.
        const char* shot = getenv("SILVAPI_SCREENSHOT");
        if (shot && max_frames > 0 && frame_no == max_frames) {
            r_capture_ppm(shot);
            fprintf(stderr, "[Shot] %s logical=%dx%d fb=%dx%d scale=%.4f\n",
                    shot, logical_w, logical_h, fb_w, fb_h,
                    logical_w ? (float)fb_w / (float)logical_w : 1.0f);
        }

        glfwSwapBuffers(window);
    }

    // Opt-in text engine diagnostics: SILVAPI_TEXT_STATS=1
    if (getenv("SILVAPI_TEXT_STATS")) {
        double elapsed = glfwGetTime() - start_time;
        if (frame_no > 0) {
            fprintf(stderr, "[Perf] %ld frames in %.3fs = %.3f ms/frame (%.0f fps)\n",
                    frame_no, elapsed, 1000.0 * elapsed / (double)frame_no,
                    (double)frame_no / elapsed);
        }
        TextStats ts = font_stats();
        size_t total = ts.shape_cache_hits + ts.shape_cache_misses;
        fprintf(stderr,
                "[Text] shape cache: %zu entries, %zu hits, %zu misses (%.1f%% hit), "
                "atlas %.1f%% full\n",
                ts.shape_cache_entries, ts.shape_cache_hits, ts.shape_cache_misses,
                total ? 100.0 * (double)ts.shape_cache_hits / (double)total : 0.0,
                100.0 * (double)ts.atlas_occupancy);
    }

    // Cleanup
    app_state_shutdown();
    layout_shutdown();
    uil_layout_shutdown();
    ui_shutdown();
    gl_backend_shutdown();
    http_client_shutdown();
    storage_shutdown();

    glfwDestroyWindow(window);
    glfwTerminate();

    return 0;
}

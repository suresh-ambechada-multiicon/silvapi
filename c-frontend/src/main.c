#include "silvapi.h"
#include "gl_backend.h"
#include "theme.h"
#include "font.h"
#include "model.h"
#include "storage.h"
#include "http_client.h"
#include "json_utils.h"
#include "fredbuf.h"
#include "panel_layout.h"
#include "ui.h"
#include "clay_renderer.h"
#include "views/views.h"
#include "GLFW/glfw3.h"

AppState g_app = {0};

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

// GLFW Callbacks
static void cursor_pos_callback(GLFWwindow* window, double xpos, double ypos) {
    (void)window;
    g_ui.mouse_x = (float)xpos;
    g_ui.mouse_y = (float)ypos;
}

static void mouse_button_callback(GLFWwindow* window, int button, int action, int mods) {
    (void)window;
    (void)mods;
    static double s_last_click_time = 0.0;
    static float s_last_click_x = 0.0f;
    static float s_last_click_y = 0.0f;

    if (button >= 0 && button < 3) {
        if (action == GLFW_PRESS) {
            g_ui.mouse_down[button] = true;
            g_ui.mouse_clicked[button] = true;

            if (button == 0) {
                double now = glfwGetTime();
                if (now - s_last_click_time < 0.35 &&
                    fabsf(g_ui.mouse_x - s_last_click_x) < 6.0f &&
                    fabsf(g_ui.mouse_y - s_last_click_y) < 6.0f) {
                    g_ui.mouse_click_count = (g_ui.mouse_click_count % 3) + 1;
                } else {
                    g_ui.mouse_click_count = 1;
                }
                s_last_click_time = now;
                s_last_click_x = g_ui.mouse_x;
                s_last_click_y = g_ui.mouse_y;
            }
        } else if (action == GLFW_RELEASE) {
            g_ui.mouse_down[button] = false;
            g_ui.mouse_released[button] = true;
        }
    }
}

static void scroll_callback(GLFWwindow* window, double xoffset, double yoffset) {
    (void)window;
    g_ui.scroll_x = (float)xoffset;
    g_ui.scroll_y = (float)yoffset;
}

static void key_callback(GLFWwindow* window, int key, int scancode, int action, int mods) {
    (void)window;
    (void)scancode;
    g_ui.ctrl_down = (mods & GLFW_MOD_CONTROL) != 0;
    g_ui.shift_down = (mods & GLFW_MOD_SHIFT) != 0;
    g_ui.alt_down = (mods & GLFW_MOD_ALT) != 0;

    if (action == GLFW_PRESS || action == GLFW_REPEAT) {
        g_ui.key_pressed = key;

        // Shortcuts
        if (g_ui.ctrl_down && key == GLFW_KEY_P) {
            g_app.show_command_palette = !g_app.show_command_palette;
            g_app.command_palette_query[0] = '\0';
            g_app.command_palette_sel_idx = 0;
        } else if (g_ui.ctrl_down && key == GLFW_KEY_ENTER) {
            app_send_active_request();
        } else if (g_ui.ctrl_down && key == GLFW_KEY_B) {
            layout_toggle_sidebar();
        } else if (g_ui.ctrl_down && g_ui.shift_down && key == GLFW_KEY_R) {
            layout_reset_default((float)g_ui.window_width, (float)g_ui.window_height);
            app_show_toast("Layout reset to default");
        } else if (g_ui.ctrl_down && key == GLFW_KEY_W) {
            app_close_tab(g_app.active_tab_index);
        } else if (g_ui.ctrl_down && key == GLFW_KEY_S) {
            storage_save_workspace(g_app.workspace);
            app_show_toast("Workspace saved");
        } else if (g_ui.ctrl_down && key == GLFW_KEY_COMMA) {
            g_app.show_settings_modal = !g_app.show_settings_modal;
        }
    }
}

static void char_callback(GLFWwindow* window, unsigned int codepoint) {
    (void)window;
    if (codepoint < 128 && g_ui.char_input_len < sizeof(g_ui.char_input) - 1) {
        g_ui.char_input[g_ui.char_input_len++] = (char)codepoint;
        g_ui.char_input[g_ui.char_input_len] = '\0';
    }
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
    GLFWwindow* window = glfwCreateWindow(win_w, win_h, "Silvapi - API Client", NULL, NULL);
    if (!window) {
        fprintf(stderr, "Failed to create GLFW window\n");
        glfwTerminate();
        return 1;
    }

    glfwMakeContextCurrent(window);
    glfwSwapInterval(1); // Enable VSync

    // Callbacks
    glfwSetCursorPosCallback(window, cursor_pos_callback);
    glfwSetMouseButtonCallback(window, mouse_button_callback);
    glfwSetScrollCallback(window, scroll_callback);
    glfwSetKeyCallback(window, key_callback);
    glfwSetCharCallback(window, char_callback);

    int fb_w, fb_h;
    glfwGetFramebufferSize(window, &fb_w, &fb_h);

    // Initialize systems
    storage_init();
    http_client_init();
    gl_backend_init(win_w, win_h, fb_w, fb_h);
    ui_init(win_w, win_h);
    clay_renderer_init(win_w, win_h);
    layout_init((float)win_w, (float)win_h);
    app_state_init();

    double last_time = glfwGetTime();

    while (!glfwWindowShouldClose(window)) {
        double current_time = glfwGetTime();
        float dt = (float)(current_time - last_time);
        last_time = current_time;

        int logical_w, logical_h;
        glfwGetWindowSize(window, &logical_w, &logical_h);
        glfwGetFramebufferSize(window, &fb_w, &fb_h);

        g_ui.window_width = logical_w;
        g_ui.window_height = logical_h;

        gl_backend_resize(logical_w, logical_h, fb_w, fb_h);
        gl_backend_begin_frame();
        ui_begin_frame(dt);

        Theme* t = theme_get_active();
        r_draw_rect(rect_make(0, 0, (float)logical_w, (float)logical_h), t->background);

        // 1. Top Bar
        float topbar_h = 42.0f;
        Rect topbar_rect = rect_make(0, 0, (float)logical_w, topbar_h);
        view_top_bar(topbar_rect);

        // 2. Docking Layout System (Sidebar, Request Editor, Response Panel, etc.)
        float main_y = topbar_h;
        float main_h = (float)logical_h - topbar_h;
        Rect layout_rect = rect_make(0, main_y, (float)logical_w, main_h);

        layout_update_bounds(g_layout.root, layout_rect, dt);
        layout_render_and_handle(g_layout.root);

        // 3. Dialogs, Modals, Menus, Tooltips
        view_dialogs();

        ui_end_frame();
        gl_backend_end_frame();

        glfwSwapBuffers(window);
        glfwPollEvents();
    }

    // Cleanup
    app_state_shutdown();
    layout_shutdown();
    clay_renderer_shutdown();
    ui_shutdown();
    gl_backend_shutdown();
    http_client_shutdown();
    storage_shutdown();

    glfwDestroyWindow(window);
    glfwTerminate();

    return 0;
}

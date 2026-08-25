#ifndef VIEWS_H
#define VIEWS_H

#include "base/base.h"
#include "app/app.h"
#include "app/model.h"
#include "app/storage.h"
#include "app/http_client.h"
#include "json/json_utils.h"
#include "ui/theme.h"
#include "ui/ui.h"

#define MAX_OPEN_TABS 32

// Dockable view ids. These are app-chosen integers handed to libs/ui's dock,
// which stores a name + render callback per id and knows nothing else about them.
enum {
    VIEW_SIDEBAR = 0,
    VIEW_REQUEST,
    VIEW_RESPONSE,
    VIEW_VARIABLES,
    VIEW_HISTORY,
    VIEW_COUNT
};

// Registers every view above with the dock. Call once after ui_init().
void app_register_dock_views(void);

// Adapts the app's KeyValue array to the kv-editor component. Declared here so
// every caller shares one adapter rather than writing its own.
struct UiKvRow;
void app_kv_row(size_t index, struct UiKvRow* out, void* user);

typedef struct {
    KeyValue** items;
    size_t*    count;
} AppKvBinding;

typedef struct {
    Workspace* workspace;
    
    // Open Request Tabs
    TabItem tabs[MAX_OPEN_TABS];
    ApiRequest* open_requests[MAX_OPEN_TABS];
    size_t tab_count;
    int active_tab_index;

    // Active Response & Execution
    HttpResponse active_response;
    bool is_sending;
    char last_sent_request_id[64];

    // Response History Cache
    HistoryEntry* history_entries;
    size_t history_count;
    bool show_history_dropdown;

    // Layout Splitters
    float sidebar_width;
    float request_split_ratio; // 0.0 to 1.0 (between request editor & response panel)
    float table_detail_height; // bottom inspector panel height

    // Sub-tab Selections
    int request_subtab;   // 0: Body, 1: Params, 2: Headers, 3: Auth, 4: Info
    int response_subtab;  // 0: Response, 1: Table, 2: Headers, 3: Timeline

    // Search and Table State
    char sidebar_search[128];
    char table_search[128];
    int table_selected_row;
    int table_selected_col;
    bool table_markdown_view;
    JsonTable* active_json_table;

    // Scroll Offsets
    float sidebar_scroll_y;
    float request_body_scroll_y;
    float request_params_scroll_y;
    float request_headers_scroll_y;
    float response_body_scroll_y;
    float response_table_scroll_y;
    float response_headers_scroll_y;
    float response_timeline_scroll_y;

    // Dialogs / Overlays
    bool show_variables_modal;
    bool show_settings_modal;
    bool show_command_palette;
    char command_palette_query[128];
    int command_palette_sel_idx;
    bool show_import_modal;
    char import_text_buffer[8192];
    int import_type; // 0: OpenAPI, 1: Postman, 2: cURL

    // Context Menu
    char context_item_id[64];
    bool is_context_folder;
    bool show_context_menu;

    // Toast notifications
    char toast_message[256];
    float toast_timer;

} AppState;

extern AppState g_app;

void app_state_init(void);
void app_state_shutdown(void);

void app_open_request(ApiRequest* req);
void app_close_tab(int tab_idx);
void app_send_active_request(void);
void app_cancel_active_request(void);
void app_show_toast(const char* message);

// Views
void view_top_bar(Rect rect);
void view_sidebar(Rect rect);
void view_request_panel(Rect rect);
void view_response_panel(Rect rect);
void view_dialogs(void);
void view_demo_gallery(Rect rect);
void view_demo_menus(Rect rect);
void view_demo_tree(Rect rect);
void view_demo_tabs(Rect rect);
void view_variables_panel(Rect rect);
void view_demo_code(Rect rect);
void view_demo_controls(Rect rect);
void view_demo_options(Rect rect);

#endif // VIEWS_H

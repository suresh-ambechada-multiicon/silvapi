#include "app/views.h"
#include "app/config.h"
#include "gfx/gfx.h"
#include "ui/layout.h"
#include "ui/components/tree.h"
#include "input/input.h"

// ---------------------------------------------------------------------------
// Collections sidebar, expressed declaratively.
//
// The previous version walked the tree with a `float* cur_y` cursor, advancing
// by hardcoded row heights (28.0f, +2.0f, indent + 16.0f) and subtracting a
// manual scroll offset before clipping. Every row width was
// `rect.w - indent - 8.0f`, so a name, a method badge, and a status dot could
// only be kept apart by arithmetic that had to agree at three separate call
// sites.
//
// Here rows are column children: the method badge is sized to its own text, the
// name takes the remaining space and ellipsizes, and scrolling is a property of
// the container rather than an offset applied by hand.
// ---------------------------------------------------------------------------

typedef struct {
    Theme* t;
} SbStyle;

// Rows are emitted recursively; the tree component owns indentation, chevrons,
// row chrome, culling, and hit-testing, so this file only decides what a row
// *says*. The previous version tracked a `float* cur_y` cursor and computed
// every indent, row rect, and scroll offset by hand.
static void emit_item(CollectionItem* item);

static bool request_matches_search(const ApiRequest* req) {
    if (g_app.sidebar_search[0] == '\0') return true;
    return strcasestr(req->name, g_app.sidebar_search) ||
           strcasestr(http_method_to_string(req->method), g_app.sidebar_search) ||
           strcasestr(req->url, g_app.sidebar_search);
}

// True when any request under this folder survives the filter, so a folder whose
// contents all filtered away is hidden rather than shown empty.
static bool folder_matches_search(const Folder* f) {
    if (g_app.sidebar_search[0] == '\0') return true;
    if (!f) return false;
    for (size_t i = 0; i < f->items_count; i++) {
        CollectionItem* ci = f->items[i];
        if (!ci) continue;
        if (ci->type == ITEM_REQUEST && ci->request && request_matches_search(ci->request)) return true;
        if (ci->type == ITEM_FOLDER && folder_matches_search(ci->folder)) return true;
    }
    return false;
}

static Color status_dot_color(int status) {
    Theme* t = theme_get_active();
    if (status <= 0) return (Color){0};
    if (status < 300) return t->success;
    if (status < 400) return t->warning;
    return t->danger;
}

static void emit_request(ApiRequest* req) {
    Theme* t = theme_get_active();
    bool selected = (g_app.tab_count > 0 &&
                     g_app.active_tab_index >= 0 &&
                     g_app.active_tab_index < (int)g_app.tab_count &&
                     strcmp(g_app.open_requests[g_app.active_tab_index]->id, req->id) == 0);

    const char* method = http_method_to_string(req->method);

    ui_tree_node(&(UiTreeNode){
        .id = req->id,
        .label = req->name,
        .badge = method,
        .badge_color = theme_get_method_color(method),
        .dot_color = status_dot_color(req->last_status),
        .selected = selected,
        .user = req,
    });

    if (ui_tree_node_clicked()) app_open_request(req);
    if (ui_tree_node_context()) {
        g_app.show_context_menu = true;
        g_app.is_context_folder = false;
        snprintf(g_app.context_item_id, sizeof(g_app.context_item_id), "%s", req->id);
        g_ui.context_menu_pos = (Vec2){ input_mouse().x, input_mouse().y };
    }
    (void)t;
}

static void emit_folder(Folder* f) {
    Theme* t = theme_get_active();

    bool open = ui_tree_node(&(UiTreeNode){
        .id = f->id,
        .label = f->name,
        .icon = f->is_expanded ? ICON_FOLDER_OPEN : ICON_FOLDER,
        .icon_color = t->warning,
        .expandable = true,
        .expanded = f->is_expanded,
        .user = f,
    });

    if (ui_tree_node_toggled()) f->is_expanded = !f->is_expanded;
    if (ui_tree_node_context()) {
        g_app.show_context_menu = true;
        g_app.is_context_folder = true;
        snprintf(g_app.context_item_id, sizeof(g_app.context_item_id), "%s", f->id);
        g_ui.context_menu_pos = (Vec2){ input_mouse().x, input_mouse().y };
    }

    if (open) {
        for (size_t i = 0; i < f->items_count; i++) emit_item(f->items[i]);
        ui_tree_pop();
    }
}

static void emit_item(CollectionItem* item) {
    if (!item) return;
    if (item->type == ITEM_FOLDER && item->folder) {
        if (!folder_matches_search(item->folder)) return;
        emit_folder(item->folder);
    } else if (item->type == ITEM_REQUEST && item->request) {
        if (!request_matches_search(item->request)) return;
        emit_request(item->request);
    }
}

static void add_request_to_collection(Collection* col) {
    ApiRequest* req = api_request_create("New Request", METHOD_GET,
                                         "https://jsonplaceholder.typicode.com/posts/1");
    CollectionItem* ci = calloc(1, sizeof(CollectionItem));
    if (!ci) return;
    ci->type = ITEM_REQUEST;
    ci->request = req;

    CollectionItem** grown = realloc(col->items,
                                     sizeof(CollectionItem*) * (col->items_count + 1));
    if (!grown) {
        free(ci);
        return;
    }
    col->items = grown;
    col->items[col->items_count++] = ci;
    app_open_request(req);
    storage_save_workspace(g_app.workspace);
}

void view_sidebar(Rect rect) {
    Theme* t = theme_get_active();
    float radius = g_config.corner_radius;

    UI_PANEL(rect, .bg = t->sidebar_bg) {
        UI_COL(.w = UI_GROW, .h = UI_GROW, .pad = g_config.pad_medium,
               .gap = g_config.gap_small) {

            UI_TEXT_INPUT(.id = "sidebar.search",
                          .buffer = g_app.sidebar_search,
                          .buffer_size = sizeof(g_app.sidebar_search),
                          .placeholder = "Search requests...",
                          .h = UI_PX(g_config.row_height),
                          .bg = t->input_bg, .fg = t->foreground,
                          .placeholder_color = t->input_placeholder,
                          .border_color = t->input_border,
                          .border_focus_color = t->input_border_focus,
                          .caret_color = t->primary,
                          .radius = radius);

            static int s_tree_sel = -1;
            ui_tree_begin(&(UiTreeProps){
                .id = "sidebar.tree",
                .row_height = g_config.row_height,
                .indent = 14.0f,
                .show_guides = true,
                .selected = &s_tree_sel,
            });

            Workspace* ws = g_app.workspace;
            if (ws) {
                for (size_t c = 0; c < ws->collections_count; c++) {
                    Collection* col = ws->collections[c];
                    if (!col) continue;

                    bool open = ui_tree_node(&(UiTreeNode){
                        .id = col->id,
                        .label = col->name,
                        .expandable = true,
                        .expanded = col->is_expanded,
                        .user = col,
                    });
                    if (ui_tree_node_toggled()) col->is_expanded = !col->is_expanded;

                    if (open) {
                        for (size_t i = 0; i < col->items_count; i++) {
                            emit_item(col->items[i]);
                        }
                        ui_tree_pop();
                    }
                }
            }

            UiTreeResult tr = ui_tree_end();

            if (getenv("SILVAPI_TREE_TRACE")) {
                static int tick;
                if (++tick % 30 == 0) {
                    fprintf(stderr, "[Tree] rows=%zu emitted=%zu selected=%d\n",
                            tr.row_count, tr.emitted, s_tree_sel);
                }
            }

            // Collection-level add stays outside the tree: it acts on the
            // workspace, not on a row.
            UI_ROW(.w = UI_GROW, .h = UI_PX(g_config.row_height),
                   .align_y = UI_CENTER) {
                if (ws && ws->collections_count > 0 &&
                    UI_BUTTON(.id = "sb.add", .icon = ICON_PLUS, .label = "New Request",
                              .icon_size = 12.0f, .h = UI_PX(22.0f),
                              .fg = t->text_muted, .fg_hover = t->primary,
                              .bg_hover = t->secondary_hover, .radius = radius)) {
                    add_request_to_collection(ws->collections[0]);
                }
            }
        }
    }
}

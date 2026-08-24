#include "views.h"
#include "gl_backend.h"

static void render_collection_item(CollectionItem* item, float* cur_y, float indent, Rect rect) {
    if (!item) return;
    Theme* t = theme_get_active();
    float item_h = 28.0f;

    if (item->type == ITEM_FOLDER) {
        Folder* f = item->folder;
        if (!f) return;

        Rect ir = rect_make(rect.x + indent, *cur_y, rect.w - indent - 8.0f, item_h);
        bool ctx = false;
        bool is_open = f->is_expanded;

        ui_tree_item(f->id, f->name, true, &f->is_expanded, METHOD_GET, 0, false, &ctx, ir);

        if (ctx) {
            g_app.show_context_menu = true;
            g_app.is_context_folder = true;
            strncpy(g_app.context_item_id, f->id, sizeof(g_app.context_item_id) - 1);
        }

        *cur_y += item_h + 2.0f;

        if (f->is_expanded && f->items) {
            for (size_t i = 0; i < f->items_count; i++) {
                render_collection_item(f->items[i], cur_y, indent + 16.0f, rect);
            }
        }
    } else if (item->type == ITEM_REQUEST) {
        ApiRequest* req = item->request;
        if (!req) return;

        // Search filter matching
        if (g_app.sidebar_search[0] != '\0') {
            if (!strcasestr(req->name, g_app.sidebar_search) &&
                !strcasestr(http_method_to_string(req->method), g_app.sidebar_search) &&
                !strcasestr(req->url, g_app.sidebar_search)) {
                return;
            }
        }

        bool is_selected = (g_app.tab_count > 0 &&
                            g_app.active_tab_index >= 0 &&
                            g_app.active_tab_index < (int)g_app.tab_count &&
                            strcmp(g_app.open_requests[g_app.active_tab_index]->id, req->id) == 0);

        Rect ir = rect_make(rect.x + indent, *cur_y, rect.w - indent - 8.0f, item_h);
        bool ctx = false;

        if (ui_tree_item(req->id, req->name, false, NULL, req->method, req->last_status, is_selected, &ctx, ir)) {
            app_open_request(req);
        }

        if (ctx) {
            g_app.show_context_menu = true;
            g_app.is_context_folder = false;
            strncpy(g_app.context_item_id, req->id, sizeof(g_app.context_item_id) - 1);
        }

        *cur_y += item_h + 2.0f;
    }
}

void view_sidebar(Rect rect) {
    Theme* t = theme_get_active();

    // Background
    r_draw_rect(rect, t->sidebar_bg);
    r_draw_line((Vec2){rect.x + rect.w, rect.y}, (Vec2){rect.x + rect.w, rect.y + rect.h}, 1.0f, t->border);

    // 1. Search Bar
    Rect search_rect = rect_make(rect.x + 8.0f, rect.y + 8.0f, rect.w - 16.0f, 28.0f);
    ui_input_text_icon("sidebar_search", ICON_SEARCH, g_app.sidebar_search, sizeof(g_app.sidebar_search), "Search requests...", search_rect);

    // 2. Collections Tree
    float tree_y = rect.y + 44.0f;
    float tree_h = rect.h - 44.0f;
    Rect tree_rect = rect_make(rect.x, tree_y, rect.w, tree_h);

    float content_h = 16.0f;
    if (g_app.workspace) {
        for (size_t c = 0; c < g_app.workspace->collections_count; c++) {
            content_h += 30.0f;
            Collection* col = g_app.workspace->collections[c];
            if (col && col->is_expanded && col->items) {
                content_h += col->items_count * 26.0f;
            }
        }
    }

    uint64_t sb_id = ui_hash_id("sidebar_tree", 0);
    float current_scroll = ui_smooth_scroll(sb_id, tree_rect, content_h, &g_app.sidebar_scroll_y);

    r_push_clip(tree_rect);
    float cur_y = tree_y + 4.0f - current_scroll;

    if (g_app.workspace) {
        for (size_t c = 0; c < g_app.workspace->collections_count; c++) {
            Collection* col = g_app.workspace->collections[c];
            if (!col) continue;

            // Collection Header
            Rect cr = rect_make(rect.x + 8.0f, cur_y, rect.w - 16.0f, 26.0f);
            bool hovered = rect_contains(cr, g_ui.mouse_x, g_ui.mouse_y);
            if (hovered && g_ui.mouse_clicked[0]) {
                col->is_expanded = !col->is_expanded;
            }

            // Collection Title + Chevron
            Rect ch_rect = rect_make(cr.x, cr.y + (cr.h - 12.0f) * 0.5f, 12.0f, 12.0f);
            r_draw_icon(col->is_expanded ? ICON_CHEVRON_DOWN : ICON_CHEVRON_RIGHT, ch_rect, t->text_muted);
            float title_h = font_get_height(g_ui.font_bold);
            font_draw_text(g_ui.font_bold, col->name, 0, cr.x + 16.0f, cr.y + (cr.h - title_h) * 0.5f, t->foreground);

            // Add Request (+) button on collection header
            Rect add_btn = rect_make(cr.x + cr.w - 20.0f, cr.y + (cr.h - 18.0f) * 0.5f, 18.0f, 18.0f);
            if (ui_button_icon(ICON_PLUS, NULL, add_btn, false)) {
                ApiRequest* new_req = api_request_create("New Request", METHOD_GET, "https://jsonplaceholder.typicode.com/posts/1");
                CollectionItem* ci = calloc(1, sizeof(CollectionItem));
                ci->type = ITEM_REQUEST;
                ci->request = new_req;
                col->items = realloc(col->items, sizeof(CollectionItem*) * (col->items_count + 1));
                col->items[col->items_count++] = ci;
                app_open_request(new_req);
                storage_save_workspace(g_app.workspace);
            }

            cur_y += 28.0f;

            // Collection Children
            if (col->is_expanded && col->items) {
                for (size_t i = 0; i < col->items_count; i++) {
                    render_collection_item(col->items[i], &cur_y, 14.0f, rect);
                }
            }
        }
    }

    r_pop_clip();
    ui_draw_scrollbar(sb_id, tree_rect, content_h);
}

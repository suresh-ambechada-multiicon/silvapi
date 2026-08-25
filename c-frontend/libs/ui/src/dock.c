#include "ui/dock.h"
#include "gfx/gfx.h"
#include "ui/theme.h"
#include "text/font.h"
#include "ui/ui.h"
#include <stdlib.h>
#include <string.h>
#include <math.h>

LayoutManager g_layout = {0};

// ---------------------------------------------------------------------------
// View registry. The application registers each dockable panel; the dock never
// names or reaches into a specific view.
// ---------------------------------------------------------------------------
typedef struct {
    const char*  name;
    DockRenderFn render;       // NULL for declarative views
    void*        user_data;
    bool         registered;
    bool         declarative;
} DockViewSlot;

static DockViewSlot s_dock_views[DOCK_MAX_VIEWS];

// Default tree composition, supplied by the app at layout_init().
static ViewKind s_default_side   = DOCK_VIEW_NONE;
static ViewKind s_default_main_a = DOCK_VIEW_NONE;
static ViewKind s_default_main_b = DOCK_VIEW_NONE;

void dock_register_view(ViewKind id, const char* name, DockRenderFn render, void* user_data) {
    if (id < 0 || id >= DOCK_MAX_VIEWS) return;
    s_dock_views[id].name       = name;
    s_dock_views[id].render     = render;
    s_dock_views[id].user_data  = user_data;
    s_dock_views[id].registered = true;
}

void dock_register_view_declarative(ViewKind id, const char* name, void* user_data) {
    if (id < 0 || id >= DOCK_MAX_VIEWS) return;
    s_dock_views[id].name        = name;
    s_dock_views[id].render      = NULL;
    s_dock_views[id].user_data   = user_data;
    s_dock_views[id].registered  = true;
    s_dock_views[id].declarative = true;
}

void dock_clear_views(void) {
    memset(s_dock_views, 0, sizeof(s_dock_views));
}

// Rebuilt every frame while walking the panel tree.
#define DOCK_MAX_VISIBLE 32
static DockViewRect s_visible[DOCK_MAX_VISIBLE];
static size_t       s_visible_count = 0;

size_t dock_visible_views(DockViewRect* out, size_t max) {
    if (!out) return 0;
    size_t n = s_visible_count < max ? s_visible_count : max;
    memcpy(out, s_visible, n * sizeof(DockViewRect));
    return n;
}

const char* layout_view_name(ViewKind view) {
    if (view < 0 || view >= DOCK_MAX_VIEWS) return "Panel";
    const char* n = s_dock_views[view].name;
    return n ? n : "Panel";
}

static PanelNode* create_leaf_node(ViewKind view) {
    PanelNode* node = calloc(1, sizeof(PanelNode));
    node->kind = NODE_LEAF;
    node->views[0] = view;
    node->view_count = 1;
    node->active_view_index = 0;
    node->collapse_spring = spring1d_make(1.0f, SPRING_CONFIG_SMOOTH);
    node->tab_indicator_spring = spring_rect_make((Rect){0, 0, 0, 0}, SPRING_CONFIG_SNAPPY);
    return node;
}

static PanelNode* create_split_node(NodeKind kind, float split_pct, PanelNode* child_a, PanelNode* child_b) {
    PanelNode* node = calloc(1, sizeof(PanelNode));
    node->kind = kind;
    node->split_pct = split_pct;
    node->split_spring = spring1d_make(split_pct, SPRING_CONFIG_SNAPPY);
    node->child_a = child_a;
    node->child_b = child_b;
    node->collapse_spring = spring1d_make(1.0f, SPRING_CONFIG_SMOOTH);
    if (child_a) child_a->parent = node;
    if (child_b) child_b->parent = node;
    return node;
}

static void free_panel_node(PanelNode* node) {
    if (!node) return;
    if (node->child_a) free_panel_node(node->child_a);
    if (node->child_b) free_panel_node(node->child_b);
    free(node);
}

void layout_reset_default(float window_w, float window_h) {
    (void)window_h;
    if (g_layout.root) {
        free_panel_node(g_layout.root);
        g_layout.root = NULL;
    }

    g_layout.sidebar_collapsed = false;
    g_layout.sidebar_width = 240.0f;
    g_layout.sidebar_spring = spring1d_make(1.0f, SPRING_CONFIG_SMOOTH);
    g_layout.dock_preview_spring = spring_rect_make((Rect){0, 0, 0, 0}, SPRING_CONFIG_BOUNCY);

    PanelNode* sidebar_leaf = create_leaf_node(s_default_side);
    sidebar_leaf->min_size_a = 180.0f;

    PanelNode* req_leaf = create_leaf_node(s_default_main_a);
    PanelNode* resp_leaf = create_leaf_node(s_default_main_b);

    PanelNode* main_split = create_split_node(NODE_SPLIT_H, 0.5f, req_leaf, resp_leaf);
    main_split->min_size_a = 280.0f;
    main_split->min_size_b = 280.0f;

    float sidebar_pct = 240.0f / (window_w > 0 ? window_w : 1280.0f);
    g_layout.root = create_split_node(NODE_SPLIT_H, sidebar_pct, sidebar_leaf, main_split);
    g_layout.root->min_size_a = 180.0f;
    g_layout.root->min_size_b = 600.0f;
}

void layout_init(float window_w, float window_h, ViewKind side, ViewKind main_a, ViewKind main_b) {
    s_default_side   = side;
    s_default_main_a = main_a;
    s_default_main_b = main_b;
    layout_reset_default(window_w, window_h);
}

void layout_shutdown(void) {
    if (g_layout.root) {
        free_panel_node(g_layout.root);
        g_layout.root = NULL;
    }
}

void layout_toggle_sidebar(void) {
    g_layout.sidebar_collapsed = !g_layout.sidebar_collapsed;
}

void layout_update_bounds(PanelNode* node, Rect rect, float dt) {
    if (!node) return;
    node->bounds = rect;

    if (node == g_layout.root) {
        // Smooth spring animation for sidebar
        float target_anim = g_layout.sidebar_collapsed ? 0.0f : 1.0f;
        spring1d_set_target(&g_layout.sidebar_spring, target_anim);
        spring1d_update(&g_layout.sidebar_spring, dt);
        spring_rect_update(&g_layout.dock_preview_spring, dt);
    }

    if (node->kind == NODE_LEAF) {
        return;
    }

    // Update split spring
    spring1d_set_target(&node->split_spring, node->split_pct);
    spring1d_update(&node->split_spring, dt);

    if (node->kind == NODE_SPLIT_H) {
        float effective_split_pct = node->split_spring.pos;
        // If left child is sidebar, apply animated spring collapse
        if (node->child_a && node->child_a->kind == NODE_LEAF && node->child_a->views[0] == s_default_side) {
            float sb_w = g_layout.sidebar_width * g_layout.sidebar_spring.pos;
            if (rect.w > 0) effective_split_pct = sb_w / rect.w;
        }

        float split_x = rect.x + rect.w * effective_split_pct;
        float splitter_w = 4.0f;

        Rect rect_a = rect_make(rect.x, rect.y, split_x - rect.x, rect.h);
        Rect rect_b = rect_make(split_x + splitter_w, rect.y, (rect.x + rect.w) - (split_x + splitter_w), rect.h);

        if (rect_a.w < 0) rect_a.w = 0;
        if (rect_b.w < 0) rect_b.w = 0;

        layout_update_bounds(node->child_a, rect_a, dt);
        layout_update_bounds(node->child_b, rect_b, dt);
    } else if (node->kind == NODE_SPLIT_V) {
        float split_y = rect.y + rect.h * node->split_spring.pos;
        float splitter_h = 4.0f;

        Rect rect_a = rect_make(rect.x, rect.y, rect.w, split_y - rect.y);
        Rect rect_b = rect_make(rect.x, split_y + splitter_h, rect.w, (rect.y + rect.h) - (split_y + splitter_h));

        if (rect_a.h < 0) rect_a.h = 0;
        if (rect_b.h < 0) rect_b.h = 0;

        layout_update_bounds(node->child_a, rect_a, dt);
        layout_update_bounds(node->child_b, rect_b, dt);
    }
}

static PanelNode* prune_empty_nodes(PanelNode* node) {
    if (!node) return NULL;
    if (node->kind == NODE_LEAF) {
        if (node->view_count == 0) {
            free(node);
            return NULL;
        }
        return node;
    }

    node->child_a = prune_empty_nodes(node->child_a);
    node->child_b = prune_empty_nodes(node->child_b);

    if (!node->child_a && node->child_b) {
        PanelNode* b = node->child_b;
        b->parent = node->parent;
        free(node);
        return b;
    }
    if (node->child_a && !node->child_b) {
        PanelNode* a = node->child_a;
        a->parent = node->parent;
        free(node);
        return a;
    }
    if (!node->child_a && !node->child_b) {
        free(node);
        return NULL;
    }
    return node;
}

static void render_leaf_content(PanelNode* leaf) {
    if (!leaf || leaf->view_count == 0 || leaf->bounds.w <= 4.0f || leaf->bounds.h <= 4.0f) return;

    ViewKind active_view = leaf->views[leaf->active_view_index];
    Theme* t = theme_get_active();

    float hdr_h = 28.0f;
    Rect hdr_rect = rect_make(leaf->bounds.x, leaf->bounds.y, leaf->bounds.w, hdr_h);
    r_draw_rect(hdr_rect, t->tabbar_bg);
    r_draw_line((Vec2){hdr_rect.x, hdr_rect.y + hdr_h}, (Vec2){hdr_rect.x + hdr_rect.w, hdr_rect.y + hdr_h}, 1.0f, t->border);

    // Tab headers
    float tab_x = hdr_rect.x + 4.0f;
    for (size_t i = 0; i < leaf->view_count; i++) {
        const char* vname = layout_view_name(leaf->views[i]);
        float tw = font_measure_text(g_ui.font_ui, vname, 0) + (leaf->view_count > 1 ? 36.0f : 24.0f);
        Rect tr = rect_make(tab_x, hdr_rect.y + 2.0f, tw, hdr_h - 4.0f);

        bool is_sel = (i == leaf->active_view_index);
        bool hov = rect_contains(tr, g_ui.mouse_x, g_ui.mouse_y);

        if (is_sel) {
            r_draw_rect_rounded(tr, 3.0f, t->panel_bg);
            r_draw_line((Vec2){tr.x + 4.0f, tr.y + tr.h - 1.0f}, (Vec2){tr.x + tr.w - 4.0f, tr.y + tr.h - 1.0f}, 2.0f, t->primary);
            font_draw_text(g_ui.font_bold, vname, 0, tr.x + 8.0f, tr.y + 4.0f, t->foreground);
        } else {
            if (hov) r_draw_rect_rounded(tr, 3.0f, t->secondary_hover);
            font_draw_text(g_ui.font_ui, vname, 0, tr.x + 8.0f, tr.y + 4.0f, t->text_muted);
        }

        // Close button if multiple views
        if (leaf->view_count > 1) {
            Rect close_r = rect_make(tr.x + tr.w - 18.0f, tr.y + 3.0f, 14.0f, 14.0f);
            bool close_hov = rect_contains(close_r, g_ui.mouse_x, g_ui.mouse_y);
            if (close_hov) {
                r_draw_rect_rounded(close_r, 2.0f, t->secondary_active);
                if (g_ui.mouse_clicked[0]) {
                    layout_remove_view_from_leaf(leaf, leaf->views[i]);
                    break;
                }
            }
            font_draw_text(g_ui.font_ui, "x", 0, close_r.x + 3.0f, close_r.y - 1.0f, close_hov ? t->foreground : t->text_dim);
        }

        if (hov && g_ui.mouse_clicked[0]) {
            leaf->active_view_index = i;
        }

        // Drag tab start
        if (hov && g_ui.mouse_down[0] && !g_layout.is_dragging_tab) {
            g_layout.is_dragging_tab = true;
            g_layout.dragged_view = leaf->views[i];
            g_layout.source_node = leaf;
        }

        tab_x += tw + 4.0f;
    }

    // Sidebar collapse button on header bar
    if (active_view == s_default_side && s_default_side != DOCK_VIEW_NONE) {
        Rect col_btn = rect_make(hdr_rect.x + hdr_rect.w - 24.0f, hdr_rect.y + 3.0f, 20.0f, 20.0f);
        if (ui_button_icon(ICON_NONE, "<", col_btn, false)) {
            layout_toggle_sidebar();
        }
    }

    Rect content_rect = rect_make(leaf->bounds.x, leaf->bounds.y + hdr_h, leaf->bounds.w, leaf->bounds.h - hdr_h);
    r_push_clip(content_rect);

    const DockViewSlot* slot = (active_view >= 0 && active_view < DOCK_MAX_VIEWS)
                                   ? &s_dock_views[active_view] : NULL;
    if (slot && slot->registered && slot->declarative) {
        // Painted later, inside the frame's Clay layout. Only the panel ground
        // is filled here so the region is not left showing stale pixels.
        r_draw_rect(content_rect, t->background);
        if (s_visible_count < DOCK_MAX_VISIBLE) {
            s_visible[s_visible_count++] = (DockViewRect){
                .id = active_view, .content_rect = content_rect, .user_data = slot->user_data
            };
        }
    } else if (slot && slot->registered && slot->render) {
        slot->render(content_rect, slot->user_data);
    } else {
        r_draw_rect(content_rect, t->background);
    }

    r_pop_clip();

    // Check hover for drag docking and draw RAD Debugger Dock Compass
    if (g_layout.is_dragging_tab && rect_contains(leaf->bounds, g_ui.mouse_x, g_ui.mouse_y)) {
        g_layout.hover_target_node = leaf;

        float mx = g_ui.mouse_x;
        float my = g_ui.mouse_y;
        Rect b = leaf->bounds;

        float cx = b.x + b.w * 0.5f;
        float cy = b.y + b.h * 0.5f;

        Rect box_center = rect_make(cx - 14.0f, cy - 14.0f, 28.0f, 28.0f);
        Rect box_left   = rect_make(cx - 48.0f, cy - 14.0f, 28.0f, 28.0f);
        Rect box_right  = rect_make(cx + 20.0f, cy - 14.0f, 28.0f, 28.0f);
        Rect box_top    = rect_make(cx - 14.0f, cy - 48.0f, 28.0f, 28.0f);
        Rect box_bottom = rect_make(cx - 14.0f, cy + 20.0f, 28.0f, 28.0f);

        float qw = b.w * 0.25f;
        float qh = b.h * 0.25f;

        Rect target_preview;
        if (rect_contains(box_left, mx, my) || mx < b.x + qw) {
            g_layout.hover_dock_zone = DOCK_LEFT;
            target_preview = rect_make(b.x, b.y, b.w * 0.5f, b.h);
        } else if (rect_contains(box_right, mx, my) || mx > b.x + b.w - qw) {
            g_layout.hover_dock_zone = DOCK_RIGHT;
            target_preview = rect_make(b.x + b.w * 0.5f, b.y, b.w * 0.5f, b.h);
        } else if (rect_contains(box_top, mx, my) || my < b.y + qh) {
            g_layout.hover_dock_zone = DOCK_TOP;
            target_preview = rect_make(b.x, b.y, b.w, b.h * 0.5f);
        } else if (rect_contains(box_bottom, mx, my) || my > b.y + b.h - qh) {
            g_layout.hover_dock_zone = DOCK_BOTTOM;
            target_preview = rect_make(b.x, b.y + b.h * 0.5f, b.w, b.h * 0.5f);
        } else {
            g_layout.hover_dock_zone = DOCK_CENTER_TAB;
            target_preview = b;
        }
        spring_rect_set_target(&g_layout.dock_preview_spring, target_preview);

        // Draw 5 Compass Buttons
        r_draw_rect_rounded_bordered(box_center, 4.0f, 1.0f, (g_layout.hover_dock_zone == DOCK_CENTER_TAB) ? t->primary : t->border, (g_layout.hover_dock_zone == DOCK_CENTER_TAB) ? t->secondary_active : t->panel_bg);
        font_draw_text(g_ui.font_bold, "+", 0, box_center.x + 9.0f, box_center.y + 4.0f, (g_layout.hover_dock_zone == DOCK_CENTER_TAB) ? t->primary : t->foreground);

        r_draw_rect_rounded_bordered(box_left, 4.0f, 1.0f, (g_layout.hover_dock_zone == DOCK_LEFT) ? t->primary : t->border, (g_layout.hover_dock_zone == DOCK_LEFT) ? t->secondary_active : t->panel_bg);
        font_draw_text(g_ui.font_bold, "<", 0, box_left.x + 9.0f, box_left.y + 4.0f, (g_layout.hover_dock_zone == DOCK_LEFT) ? t->primary : t->foreground);

        r_draw_rect_rounded_bordered(box_right, 4.0f, 1.0f, (g_layout.hover_dock_zone == DOCK_RIGHT) ? t->primary : t->border, (g_layout.hover_dock_zone == DOCK_RIGHT) ? t->secondary_active : t->panel_bg);
        font_draw_text(g_ui.font_bold, ">", 0, box_right.x + 9.0f, box_right.y + 4.0f, (g_layout.hover_dock_zone == DOCK_RIGHT) ? t->primary : t->foreground);

        r_draw_rect_rounded_bordered(box_top, 4.0f, 1.0f, (g_layout.hover_dock_zone == DOCK_TOP) ? t->primary : t->border, (g_layout.hover_dock_zone == DOCK_TOP) ? t->secondary_active : t->panel_bg);
        font_draw_text(g_ui.font_bold, "^", 0, box_top.x + 9.0f, box_top.y + 4.0f, (g_layout.hover_dock_zone == DOCK_TOP) ? t->primary : t->foreground);

        r_draw_rect_rounded_bordered(box_bottom, 4.0f, 1.0f, (g_layout.hover_dock_zone == DOCK_BOTTOM) ? t->primary : t->border, (g_layout.hover_dock_zone == DOCK_BOTTOM) ? t->secondary_active : t->panel_bg);
        font_draw_text(g_ui.font_bold, "v", 0, box_bottom.x + 9.0f, box_bottom.y + 4.0f, (g_layout.hover_dock_zone == DOCK_BOTTOM) ? t->primary : t->foreground);
    }
}

static void handle_splitter(PanelNode* node) {
    if (!node || node->kind == NODE_LEAF) return;
    Theme* t = theme_get_active();

    Rect r = node->bounds;
    Rect handle_r;
    float splitter_size = 4.0f;

    if (node->kind == NODE_SPLIT_H) {
        float effective_pct = node->split_spring.pos;
        if (node->child_a && node->child_a->kind == NODE_LEAF && node->child_a->views[0] == s_default_side) {
            effective_pct = (g_layout.sidebar_width * g_layout.sidebar_spring.pos) / (r.w > 0 ? r.w : 1.0f);
        }

        float sx = r.x + r.w * effective_pct;
        handle_r = rect_make(sx - 2.0f, r.y, splitter_size + 4.0f, r.h);

        bool hov = rect_contains(handle_r, g_ui.mouse_x, g_ui.mouse_y);
        bool is_dragging = (g_layout.dragging_splitter_node == node);

        // Draw subtle separator line
        Color line_col = is_dragging ? t->primary : (hov ? t->secondary_active : t->border);
        r_draw_rect(rect_make(sx, r.y, 1.0f, r.h), line_col);

        if (hov && g_ui.mouse_clicked[0]) {
            g_layout.dragging_splitter_node = node;
            g_layout.drag_start_mouse = (Vec2){g_ui.mouse_x, g_ui.mouse_y};
            g_layout.drag_start_split = node->split_pct;
        }

        if (is_dragging) {
            if (g_ui.mouse_down[0]) {
                float dx = g_ui.mouse_x - g_layout.drag_start_mouse.x;
                float new_split_px = (r.w * g_layout.drag_start_split) + dx;

                // Specific sidebar drag update
                if (node->child_a && node->child_a->kind == NODE_LEAF && node->child_a->views[0] == s_default_side) {
                    if (new_split_px < 140.0f) new_split_px = 140.0f;
                    if (new_split_px > 500.0f) new_split_px = 500.0f;
                    g_layout.sidebar_width = new_split_px;
                    node->split_pct = new_split_px / r.w;
                } else {
                    float min_a = node->min_size_a > 0 ? node->min_size_a : 150.0f;
                    float min_b = node->min_size_b > 0 ? node->min_size_b : 150.0f;
                    if (new_split_px < min_a) new_split_px = min_a;
                    if (new_split_px > r.w - min_b) new_split_px = r.w - min_b;
                    node->split_pct = new_split_px / r.w;
                }
            } else {
                g_layout.dragging_splitter_node = NULL;
            }
        }
    } else if (node->kind == NODE_SPLIT_V) {
        float sy = r.y + r.h * node->split_spring.pos;
        handle_r = rect_make(r.x, sy - 2.0f, r.w, splitter_size + 4.0f);

        bool hov = rect_contains(handle_r, g_ui.mouse_x, g_ui.mouse_y);
        bool is_dragging = (g_layout.dragging_splitter_node == node);

        Color line_col = is_dragging ? t->primary : (hov ? t->secondary_active : t->border);
        r_draw_rect(rect_make(r.x, sy, r.w, 1.0f), line_col);

        if (hov && g_ui.mouse_clicked[0]) {
            g_layout.dragging_splitter_node = node;
            g_layout.drag_start_mouse = (Vec2){g_ui.mouse_x, g_ui.mouse_y};
            g_layout.drag_start_split = node->split_pct;
        }

        if (is_dragging) {
            if (g_ui.mouse_down[0]) {
                float dy = g_ui.mouse_y - g_layout.drag_start_mouse.y;
                float new_split_py = (r.h * g_layout.drag_start_split) + dy;
                float min_a = node->min_size_a > 0 ? node->min_size_a : 100.0f;
                float min_b = node->min_size_b > 0 ? node->min_size_b : 100.0f;
                if (new_split_py < min_a) new_split_py = min_a;
                if (new_split_py > r.h - min_b) new_split_py = r.h - min_b;
                node->split_pct = new_split_py / r.h;
            } else {
                g_layout.dragging_splitter_node = NULL;
            }
        }
    }
}

void layout_render_and_handle(PanelNode* node) {
    // The root call starts a fresh frame's worth of visible declarative views.
    if (node && node == g_layout.root) s_visible_count = 0;

    if (!node) return;

    if (node->kind == NODE_LEAF) {
        render_leaf_content(node);
    } else {
        layout_render_and_handle(node->child_a);
        layout_render_and_handle(node->child_b);
        handle_splitter(node);
    }

    // Top-level drag and drop overlay handling
    if (node == g_layout.root && g_layout.is_dragging_tab) {
        Theme* t = theme_get_active();

        // Draw floating drag badge following cursor
        const char* drag_name = layout_view_name(g_layout.dragged_view);
        float tw = font_measure_text(g_ui.font_bold, drag_name, 0);
        Rect drag_pill = rect_make(g_ui.mouse_x + 10.0f, g_ui.mouse_y + 10.0f, tw + 24.0f, 26.0f);
        r_draw_rect_rounded_bordered(drag_pill, 4.0f, 1.0f, t->primary, t->panel_bg);
        font_draw_text(g_ui.font_bold, drag_name, 0, drag_pill.x + 12.0f, drag_pill.y + 4.0f, t->primary);

        // If mouse is released, complete docking
        if (g_ui.mouse_released[0]) {
            if (g_layout.hover_target_node && g_layout.hover_dock_zone != DOCK_NONE) {
                layout_dock_view(g_layout.hover_target_node, g_layout.dragged_view, g_layout.hover_dock_zone);
            }
            g_layout.is_dragging_tab = false;
            g_layout.hover_target_node = NULL;
            g_layout.hover_dock_zone = DOCK_NONE;
        } else if (g_layout.hover_target_node && g_layout.hover_dock_zone != DOCK_NONE) {
            // Draw RAD Debugger style spring-morphing translucent drop preview
            r_draw_rect_rounded_bordered(g_layout.dock_preview_spring.pos, 6.0f, 2.0f, t->primary, color_with_alpha(t->primary, 0.25f));
        }
    }
}

void layout_remove_view_from_leaf(PanelNode* leaf, ViewKind view) {
    if (!leaf || leaf->kind != NODE_LEAF) return;
    for (size_t i = 0; i < leaf->view_count; i++) {
        if (leaf->views[i] == view) {
            for (size_t k = i; k + 1 < leaf->view_count; k++) {
                leaf->views[k] = leaf->views[k + 1];
            }
            leaf->view_count--;
            if (leaf->active_view_index >= leaf->view_count && leaf->view_count > 0) {
                leaf->active_view_index = leaf->view_count - 1;
            }
            break;
        }
    }
}

void layout_dock_view(PanelNode* target_leaf, ViewKind view, DockZone zone) {
    if (!target_leaf || target_leaf->kind != NODE_LEAF) return;

    // Remove from source leaf
    if (g_layout.source_node) {
        layout_remove_view_from_leaf(g_layout.source_node, view);
    }

    if (zone == DOCK_CENTER_TAB) {
        // Add as tab
        if (target_leaf->view_count < 8) {
            target_leaf->views[target_leaf->view_count] = view;
            target_leaf->active_view_index = target_leaf->view_count;
            target_leaf->view_count++;
        }
    } else {
        // Split target leaf
        PanelNode* new_leaf = create_leaf_node(view);
        PanelNode* orig_leaf_clone = create_leaf_node(target_leaf->views[0]);
        for (size_t i = 1; i < target_leaf->view_count; i++) {
            orig_leaf_clone->views[i] = target_leaf->views[i];
        }
        orig_leaf_clone->view_count = target_leaf->view_count;
        orig_leaf_clone->active_view_index = target_leaf->active_view_index;

        if (zone == DOCK_LEFT) {
            target_leaf->kind = NODE_SPLIT_H;
            target_leaf->split_pct = 0.5f;
            target_leaf->child_a = new_leaf;
            target_leaf->child_b = orig_leaf_clone;
            target_leaf->view_count = 0;
        } else if (zone == DOCK_RIGHT) {
            target_leaf->kind = NODE_SPLIT_H;
            target_leaf->split_pct = 0.5f;
            target_leaf->child_a = orig_leaf_clone;
            target_leaf->child_b = new_leaf;
            target_leaf->view_count = 0;
        } else if (zone == DOCK_TOP) {
            target_leaf->kind = NODE_SPLIT_V;
            target_leaf->split_pct = 0.5f;
            target_leaf->child_a = new_leaf;
            target_leaf->child_b = orig_leaf_clone;
            target_leaf->view_count = 0;
        } else if (zone == DOCK_BOTTOM) {
            target_leaf->kind = NODE_SPLIT_V;
            target_leaf->split_pct = 0.5f;
            target_leaf->child_a = orig_leaf_clone;
            target_leaf->child_b = new_leaf;
            target_leaf->view_count = 0;
        }

        if (target_leaf->child_a) target_leaf->child_a->parent = target_leaf;
        if (target_leaf->child_b) target_leaf->child_b->parent = target_leaf;
    }

    g_layout.root = prune_empty_nodes(g_layout.root);
    if (!g_layout.root) {
        layout_reset_default((float)g_ui.window_width, (float)g_ui.window_height);
    }
}


// Depth-first search for a leaf already hosting `view`; failing that, the leaf
// with the largest area, since that is the one a user would expect a newly
// surfaced panel to appear in.
static PanelNode* find_leaf_hosting(PanelNode* node, ViewKind view) {
    if (!node) return NULL;
    if (node->kind == NODE_LEAF) {
        for (size_t i = 0; i < node->view_count; i++) {
            if (node->views[i] == view) return node;
        }
        return NULL;
    }
    PanelNode* a = find_leaf_hosting(node->child_a, view);
    return a ? a : find_leaf_hosting(node->child_b, view);
}

static PanelNode* largest_leaf(PanelNode* node) {
    if (!node) return NULL;
    if (node->kind == NODE_LEAF) return node;

    PanelNode* a = largest_leaf(node->child_a);
    PanelNode* b = largest_leaf(node->child_b);
    if (!a) return b;
    if (!b) return a;
    float area_a = a->bounds.w * a->bounds.h;
    float area_b = b->bounds.w * b->bounds.h;
    return area_a >= area_b ? a : b;
}

bool layout_set_active_view(PanelNode* root, ViewKind view) {
    if (!root || view < 0 || view >= DOCK_MAX_VIEWS) return false;

    PanelNode* leaf = find_leaf_hosting(root, view);
    if (leaf) {
        for (size_t i = 0; i < leaf->view_count; i++) {
            if (leaf->views[i] == view) {
                leaf->active_view_index = i;
                return true;
            }
        }
        return false;
    }

    leaf = largest_leaf(root);
    if (!leaf) return false;
    if (leaf->view_count >= 8) return false;

    leaf->views[leaf->view_count] = view;
    leaf->active_view_index = leaf->view_count;
    leaf->view_count++;
    return true;
}

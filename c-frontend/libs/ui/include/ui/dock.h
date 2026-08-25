#ifndef UI_DOCK_H
#define UI_DOCK_H

#include "base/base.h"
#include "ui/spring.h"
#include <stdbool.h>

// A dockable view is identified by an opaque integer the application chooses.
// The dock knows nothing about what a view contains: the app registers a name
// and a render callback, and the dock calls it with the content rect.
#define DOCK_MAX_VIEWS 32

typedef int ViewKind;

typedef void (*DockRenderFn)(Rect content_rect, void* user_data);

void dock_register_view(ViewKind id, const char* name, DockRenderFn render, void* user_data);

// A view that emits declarative layout instead of drawing immediately. The dock
// still owns its tab strip and computes its content rect, but does not call it:
// declarative content has to be emitted inside the frame's single Clay layout,
// which runs after the dock has painted. Query dock_visible_views() for the
// rects and emit there.
void dock_register_view_declarative(ViewKind id, const char* name, void* user_data);

void dock_clear_views(void);

typedef struct {
    ViewKind id;
    Rect     content_rect;
    void*    user_data;
} DockViewRect;

// Content rects of the declarative views visible this frame, valid after
// layout_render_and_handle(). Returns how many were written.
size_t dock_visible_views(DockViewRect* out, size_t max);

typedef enum {
    NODE_LEAF = 0,
    NODE_SPLIT_H,  // Split horizontally (left / right)
    NODE_SPLIT_V   // Split vertically (top / bottom)
} NodeKind;

typedef enum {
    DOCK_NONE = 0,
    DOCK_LEFT,
    DOCK_RIGHT,
    DOCK_TOP,
    DOCK_BOTTOM,
    DOCK_CENTER_TAB
} DockZone;

typedef struct PanelNode PanelNode;

struct PanelNode {
    NodeKind kind;
    Rect bounds;
    
    // Split node members
    float split_pct;        // 0.0 to 1.0
    Spring1D split_spring;  // Spring-interpolated split ratio
    PanelNode* child_a;     // Left or Top
    PanelNode* child_b;     // Right or Bottom
    PanelNode* parent;
    
    // Leaf node members
    ViewKind views[8];
    size_t view_count;
    size_t active_view_index;
    SpringRect tab_indicator_spring;
    
    // Animation & collapse
    bool is_collapsed;
    Spring1D collapse_spring;
    float min_size_a;
    float min_size_b;
};

typedef struct {
    PanelNode* root;
    
    // Drag and drop docking state
    bool is_dragging_tab;
    ViewKind dragged_view;
    PanelNode* source_node;
    PanelNode* hover_target_node;
    DockZone hover_dock_zone;
    SpringRect dock_preview_spring;
    
    // Active splitter drag
    PanelNode* dragging_splitter_node;
    Vec2 drag_start_mouse;
    float drag_start_split;
    
    // Sidebar specific
    bool sidebar_collapsed;
    float sidebar_width;
    Spring1D sidebar_spring; // 0.0 (collapsed) .. 1.0 (expanded) with fluid spring physics
} LayoutManager;

extern LayoutManager g_layout;

// The default layout tree is application policy, so the app names the three
// views it wants: a collapsible side panel plus two panels split beside it.
// Pass DOCK_VIEW_NONE for `side` to build a layout with no side panel.
#define DOCK_VIEW_NONE (-1)

void layout_init(float window_w, float window_h, ViewKind side, ViewKind main_a, ViewKind main_b);
void layout_shutdown(void);
void layout_reset_default(float window_w, float window_h);
void layout_update_bounds(PanelNode* node, Rect rect, float dt);
void layout_render_and_handle(PanelNode* node);

// Docking operations
void layout_dock_view(PanelNode* target_leaf, ViewKind view, DockZone zone);
void layout_remove_view_from_leaf(PanelNode* leaf, ViewKind view);

// Brings `view` to the front of whichever leaf hosts it, adding it to the
// largest leaf when no panel hosts it yet. Lets a caller surface a registered
// view without knowing the panel tree's shape.
bool layout_set_active_view(PanelNode* root, ViewKind view);
void layout_toggle_sidebar(void);

// View name and icon helper
const char* layout_view_name(ViewKind view);

#endif // UI_DOCK_H

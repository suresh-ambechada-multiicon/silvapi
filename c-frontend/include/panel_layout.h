#ifndef PANEL_LAYOUT_H
#define PANEL_LAYOUT_H

#include "silvapi.h"
#include <stdbool.h>

typedef enum {
    VIEW_SIDEBAR = 0,
    VIEW_REQUEST,
    VIEW_RESPONSE,
    VIEW_VARIABLES,
    VIEW_HISTORY,
    VIEW_COUNT
} ViewKind;

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
    PanelNode* child_a;     // Left or Top
    PanelNode* child_b;     // Right or Bottom
    PanelNode* parent;
    
    // Leaf node members
    ViewKind views[8];
    size_t view_count;
    size_t active_view_index;
    
    // Animation & collapse
    bool is_collapsed;
    float collapse_anim;    // 0.0 = collapsed, 1.0 = expanded
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
    Rect dock_preview_rect;
    
    // Active splitter drag
    PanelNode* dragging_splitter_node;
    Vec2 drag_start_mouse;
    float drag_start_split;
    
    // Sidebar specific
    bool sidebar_collapsed;
    float sidebar_width;
    float sidebar_anim_pct; // 0.0 (collapsed) .. 1.0 (expanded)
} LayoutManager;

extern LayoutManager g_layout;

void layout_init(float window_w, float window_h);
void layout_shutdown(void);
void layout_reset_default(float window_w, float window_h);
void layout_update_bounds(PanelNode* node, Rect rect, float dt);
void layout_render_and_handle(PanelNode* node);

// Docking operations
void layout_dock_view(PanelNode* target_leaf, ViewKind view, DockZone zone);
void layout_remove_view_from_leaf(PanelNode* leaf, ViewKind view);
void layout_toggle_sidebar(void);

// View name and icon helper
const char* layout_view_name(ViewKind view);

#endif // PANEL_LAYOUT_H

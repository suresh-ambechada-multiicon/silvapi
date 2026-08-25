#ifndef UI_COMPONENTS_TREE_H
#define UI_COMPONENTS_TREE_H

#include "ui/layout.h"
#include "ui/style.h"

// ---------------------------------------------------------------------------
// Tree view.
//
// Push-style, like the surrounding immediate-mode code, because a tree's shape
// lives in the caller's data structure and copying it into a node array every
// frame is both wasteful and a second source of truth:
//
//     ui_tree_begin(&(UiTreeProps){ .id = "files", .selected = &sel });
//     for (each collection) {
//         if (ui_tree_node(&(UiTreeNode){ .label = c->name, .expandable = true,
//                                         .expanded = c->open })) {
//             for (each child) ui_tree_node(&(UiTreeNode){ .label = ch->name });
//             ui_tree_pop();
//         }
//         if (ui_tree_node_toggled()) c->open = !c->open;
//     }
//     UiTreeResult r = ui_tree_end();
//
// Rows outside the viewport are NOT emitted: the component tracks a running y
// and replaces skipped runs with a single spacer, so an expanded tree with
// thousands of rows emits only what is visible. The caller still walks its own
// data, which is cheap; what is avoided is thousands of layout elements.
// ---------------------------------------------------------------------------

typedef struct {
    const char* id;          // stable per node; used for hit-testing
    const char* label;

    IconType icon;
    Color    icon_color;     // a=0 uses the theme default

    bool expandable;
    bool expanded;
    bool selected;           // caller-driven selection, in addition to `selected` ordinal
    bool disabled;

    // Optional trailing decorations, laid out in this order.
    const char* badge;       // pill sized to its own text, e.g. an HTTP method
    Color       badge_color;
    const char* value;       // right-aligned trailing text, e.g. a size
    Color       dot_color;   // a>0 draws a status dot

    void* user;
} UiTreeNode;

typedef struct {
    const char* id;
    const char* class;
    const char* row_class;

    float row_height;   // 0 = derived
    float indent;       // per depth level; 0 = 14px
    float max_height;   // 0 = fill the parent

    bool show_guides;   // vertical rule per indent level
    bool striped;

    int* selected;      // row ordinal, -1 for none
} UiTreeProps;

typedef struct {
    int clicked;        // row ordinal, -1
    int activated;      // double-clicked
    int context;        // right-clicked
    int toggled;        // expand/collapse changed

    size_t row_count;   // rows the caller pushed
    size_t emitted;     // rows actually laid out; the virtualization receipt
} UiTreeResult;

void ui_tree_begin(const UiTreeProps* props);

// Emits one node. Returns true when the caller should walk its children, i.e.
// the node is expandable AND expanded. Every true must be matched by
// ui_tree_pop().
bool ui_tree_node(const UiTreeNode* node);
void ui_tree_pop(void);

UiTreeResult ui_tree_end(void);

// Interaction for the node most recently passed to ui_tree_node(). Valid until
// the next ui_tree_node() call.
bool ui_tree_node_clicked(void);
bool ui_tree_node_activated(void);
bool ui_tree_node_context(void);
bool ui_tree_node_toggled(void);   // the chevron or the row toggled expansion
bool ui_tree_node_hovered(void);

#endif // UI_COMPONENTS_TREE_H

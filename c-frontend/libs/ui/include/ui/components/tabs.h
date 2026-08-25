#ifndef UI_COMPONENTS_TABS_H
#define UI_COMPONENTS_TABS_H

#include "ui/layout.h"
#include "ui/style.h"

// ---------------------------------------------------------------------------
// Tab strip.
//
// Two shapes, one component:
//   document tabs  closable, reorderable, dirty markers, overflow menu
//   sub-tabs       underlined, no close, sized to content
//
// A tab's leading decoration is a plain `badge` string plus colour rather than
// an application enum. The widget this replaces took an app-level HttpMethod
// directly, which is what forced the whole widget library to depend on the
// application's model header.
//
// Overflow: when the tabs do not fit, the strip scrolls horizontally and a
// trailing chevron reports a request to show the full list. The component does
// not open that list itself -- that is a menu, and the caller already has one.
// ---------------------------------------------------------------------------

typedef enum {
    UI_TABS_DOCUMENT = 0,   // filled tab bodies, close buttons
    UI_TABS_UNDERLINE,      // flat, active tab underlined
} UiTabsVariant;

typedef struct {
    const char* id;          // stable; used for hit-testing and drag tracking
    const char* label;

    IconType    icon;
    Color       icon_color;

    const char* badge;       // leading pill, e.g. an HTTP method
    Color       badge_color;

    bool        dirty;       // unsaved marker; shows a dot until the tab is hovered
    bool        closable;
    bool        disabled;

    void*       user;
} UiTab;

typedef struct {
    const char* id;
    const char* class;
    const char* tab_class;

    UiTabsVariant variant;

    const UiTab* tabs;
    size_t       tab_count;

    int* active;             // index of the active tab

    float height;            // 0 = derived
    // Tabs are sized to their content and then clamped to this range, which is
    // what keeps a long request name from starving its neighbours while a short
    // one still gets a usable hit target. 0 = 110 / 220.
    float min_tab_width;
    float max_tab_width;

    bool show_add;           // trailing + button
    bool show_overflow;      // trailing chevron once the tabs stop fitting
    bool reorderable;        // drag to reorder
    bool close_on_middle;    // middle-click closes
} UiTabsProps;

typedef struct {
    int  clicked;        // index, -1
    int  closed;         // index the user asked to close, -1
    int  context;        // right-clicked index, -1
    bool add_clicked;
    bool overflow_clicked;

    // A completed drag. The caller performs the move; the component only
    // reports it, because only the caller owns the tab array.
    int  reorder_from;   // -1 when no reorder happened
    int  reorder_to;

    bool overflowing;    // tabs exceed the available width
} UiTabsResult;

UiTabsResult ui_tabs(const UiTabsProps* props);

#endif // UI_COMPONENTS_TABS_H

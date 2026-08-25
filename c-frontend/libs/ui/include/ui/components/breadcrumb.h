#ifndef UI_COMPONENTS_BREADCRUMB_H
#define UI_COMPONENTS_BREADCRUMB_H

#include "ui/layout.h"
#include "ui/style.h"

// ---------------------------------------------------------------------------
// Breadcrumb path bar.
//
// Each segment is clickable (navigate there) and separately reports a chevron
// click (list that segment's siblings). The component does not open that list --
// it is a menu, and the caller already has one.
//
// Overflow collapses from the LEFT, keeping the leading root and an ellipsis,
// because the tail of a path is what identifies where you are. Trimming the
// right would hide exactly the segment the user is looking at.
// ---------------------------------------------------------------------------

typedef struct {
    const char* id;
    const char* label;
    IconType    icon;
    Color       icon_color;
    void*       user;
} UiCrumb;

typedef struct {
    const char* id;
    const char* class;
    const char* segment_class;

    const UiCrumb* crumbs;
    size_t         crumb_count;

    // Segment whose dropdown is currently open; -1 for none. Drives which
    // chevron reads as active.
    int open_segment;

    float height;          // 0 = 26
    float max_width;       // 0 = fill the parent
    bool  show_chevrons;   // per-segment dropdown affordance
    bool  editable;        // trailing space acts as a click target for path entry
} UiBreadcrumbProps;

typedef struct {
    int  clicked;          // segment index navigated to, -1
    int  chevron_clicked;  // segment whose dropdown was requested, -1
    bool background_clicked; // empty area, for switching to path entry
    size_t first_visible;  // first segment shown after left-collapse
    bool overflowed;
} UiBreadcrumbResult;

UiBreadcrumbResult ui_breadcrumb(const UiBreadcrumbProps* props);

#endif // UI_COMPONENTS_BREADCRUMB_H

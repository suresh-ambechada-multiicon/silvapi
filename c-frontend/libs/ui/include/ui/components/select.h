#ifndef UI_COMPONENTS_SELECT_H
#define UI_COMPONENTS_SELECT_H

#include "ui/layout.h"
#include "ui/style.h"

// ---------------------------------------------------------------------------
// Select trigger.
//
// Shows the current value and reports that it was clicked. It does NOT own the
// list: the caller opens a ui_menu next to it. That split is deliberate --
// options lists, submenus, and command palettes are all the same menu, and a
// select that embedded its own list would be a fourth implementation of it.
// ---------------------------------------------------------------------------

typedef struct {
    const char* id;
    int32_t     index;

    const char* value;       // current selection, shown in the trigger
    const char* placeholder; // shown when value is NULL or empty

    const char* class;
    UiSize w;                // default UI_GROW
    float  height;           // 0 = 24

    bool open;               // caller-owned; drives the chevron and focus ring
    bool disabled;
} UiSelectProps;

// Returns true on the frame the trigger is clicked.
bool ui_select(UiSelectProps props);
#define UI_SELECT(...) ui_select((UiSelectProps){ __VA_ARGS__ })

#endif // UI_COMPONENTS_SELECT_H

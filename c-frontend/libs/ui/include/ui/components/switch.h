#ifndef UI_COMPONENTS_SWITCH_H
#define UI_COMPONENTS_SWITCH_H

#include "ui/layout.h"
#include "ui/style.h"

// ---------------------------------------------------------------------------
// Toggle switch.
//
// Distinct from a checkbox: a checkbox states a fact ("include hidden files"),
// a switch performs an action with immediate effect ("animations on"). They read
// differently to a user, so they are different components rather than one with
// a style flag.
// ---------------------------------------------------------------------------

typedef struct {
    const char* id;
    int32_t     index;
    bool*       value;

    const char* class;      // track overrides
    const char* knob_class;

    float width;            // 0 = 26
    float height;           // 0 = 14
    bool  disabled;
} UiSwitchProps;

// Returns true on the frame it is toggled.
bool ui_switch(UiSwitchProps props);
#define UI_SWITCH(...) ui_switch((UiSwitchProps){ __VA_ARGS__ })

#endif // UI_COMPONENTS_SWITCH_H

#ifndef UI_COMPONENTS_STEPPER_H
#define UI_COMPONENTS_STEPPER_H

#include "ui/layout.h"
#include "ui/style.h"

// ---------------------------------------------------------------------------
// Numeric stepper: [-] value [+]
//
// For a value with a natural increment where the exact number matters -- a font
// size, a timeout. A slider is for a magnitude the user judges by eye; a stepper
// is for one they need to read and set precisely.
// ---------------------------------------------------------------------------

typedef struct {
    const char* id;
    int32_t     index;
    float*      value;

    float min;
    float max;
    float step;        // 0 = 1

    int   decimals;    // digits after the point in the label
    const char* suffix;// appended to the label, e.g. "px" or "ms"

    const char* class;
    float width;       // 0 = 108
    float height;      // 0 = 24

    bool  disabled;
    bool  show_reset;  // leading revert affordance
    float reset_value;
} UiStepperProps;

typedef struct {
    bool changed;
    bool reset_clicked;
} UiStepperResult;

UiStepperResult ui_stepper(UiStepperProps props);
#define UI_STEPPER(...) ui_stepper((UiStepperProps){ __VA_ARGS__ })

#endif // UI_COMPONENTS_STEPPER_H

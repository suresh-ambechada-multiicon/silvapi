#ifndef UI_COMPONENTS_SLIDER_H
#define UI_COMPONENTS_SLIDER_H

#include "ui/layout.h"
#include "ui/style.h"

// ---------------------------------------------------------------------------
// Slider.
//
// Dragging is tracked against the track's rect from the previous frame, because
// a drag spans frames and the pointer may leave the element while the button is
// still held -- hit-testing per frame would drop the drag the moment the cursor
// slipped off the track.
// ---------------------------------------------------------------------------

typedef struct {
    const char* id;
    int32_t     index;
    float*      value;

    float min;
    float max;
    float step;         // 0 = continuous

    const char* class;
    UiSize w;           // default UI_GROW
    float  height;      // 0 = 16
    float  track_height;// 0 = 4

    bool  disabled;
    bool  show_value;
    const char* format; // printf format for the value label; default "%.0f"
} UiSliderProps;

// Returns true while the value is changing.
bool ui_slider(UiSliderProps props);
#define UI_SLIDER(...) ui_slider((UiSliderProps){ __VA_ARGS__ })

#endif // UI_COMPONENTS_SLIDER_H

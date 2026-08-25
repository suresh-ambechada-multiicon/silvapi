#include "ui/components/switch.h"
#include "input/input.h"

#define SWITCH_TRACK_DEFAULT "row items-center rounded-full"
#define SWITCH_KNOB_DEFAULT  "rounded-full"

bool ui_switch(UiSwitchProps p) {
    if (!p.value) return false;

    const UiStyle* ts = ui_style_merge(SWITCH_TRACK_DEFAULT, p.class);
    const UiStyle* ks = ui_style_merge(SWITCH_KNOB_DEFAULT, p.knob_class);

    Color fg, muted, primary, primary_fg;
    if (!ui_style_get_token("fg", &fg))                fg = (Color){1, 1, 1, 1};
    if (!ui_style_get_token("muted", &muted))          muted = fg;
    if (!ui_style_get_token("primary", &primary))      primary = fg;
    if (!ui_style_get_token("primary-fg", &primary_fg)) primary_fg = fg;

    float w = p.width  > 0.0f ? p.width  : 26.0f;
    float h = p.height > 0.0f ? p.height : 14.0f;
    float knob = h - 4.0f;

    bool on = *p.value;
    bool hovered = p.id ? uil_element_hovered(p.id, p.index) : false;
    bool toggled = false;

    if (hovered && !p.disabled && input_mouse_pressed(INPUT_MOUSE_LEFT)) {
        *p.value = !on;
        on = *p.value;
        toggled = true;
    }

    Color track_bg = p.disabled ? color_with_alpha(muted, 0.20f)
                   : on         ? primary
                                : color_with_alpha(muted, 0.35f);
    Color knob_bg = p.disabled ? color_with_alpha(fg, 0.35f)
                  : on         ? primary_fg
                               : fg;
    if (hovered && !p.disabled && !on) {
        track_bg = color_with_alpha(muted, 0.50f);
    }

    // The knob's position is expressed as alignment rather than an offset, so it
    // stays correct at any track width the caller asks for.
    UiBoxDesc track = ui_style_box(ts, p.id, p.index, UI_STATE_BASE);
    track.w = UI_PX(w);
    track.h = UI_PX(h);
    track.pad_x = 2.0f;
    track.align_x = on ? UI_END : UI_START;
    track.align_y = UI_CENTER;
    track.bg = track_bg;
    track.radius = h * 0.5f;

    uil__open(0 /* row */, track);
    UiBoxDesc kb = ui_style_box(ks, NULL, 0, UI_STATE_BASE);
    kb.w = UI_PX(knob);
    kb.h = UI_PX(knob);
    kb.radius = knob * 0.5f;
    kb.bg = knob_bg;
    uil__open(0, kb);
    uil__close();
    uil__close();

    return toggled;
}

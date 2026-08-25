#include "ui/components/stepper.h"
#include "input/input.h"

#define STEPPER_CLASS_DEFAULT "row items-center rounded-sm bg-input"

// Repeat behaviour: holding a button keeps stepping after an initial delay, the
// way a spinner is expected to. Without it, adjusting a value by 20 needs 20
// clicks.
#define STEPPER_REPEAT_DELAY  0.35
#define STEPPER_REPEAT_RATE   0.045

typedef struct {
    uint64_t key;
    int      held;        // -1 decrement, +1 increment, 0 none
    double   next_fire;
} StepperState;

#define STEPPER_SLOTS 16
static StepperState s_steppers[STEPPER_SLOTS];

static uint64_t stepper_key(const char* id, int32_t index) {
    uint64_t h = 1469598103934665603ULL;
    for (const char* p = id ? id : "stepper"; *p; p++) {
        h ^= (uint8_t)*p;
        h *= 1099511628211ULL;
    }
    h ^= (uint64_t)(uint32_t)index * 1099511628211ULL;
    return h ? h : 1;
}

static StepperState* stepper_state(uint64_t key) {
    for (size_t i = 0; i < STEPPER_SLOTS; i++) {
        if (s_steppers[i].key == key) return &s_steppers[i];
    }
    for (size_t i = 0; i < STEPPER_SLOTS; i++) {
        if (s_steppers[i].key == 0) {
            s_steppers[i].key = key;
            s_steppers[i].held = 0;
            return &s_steppers[i];
        }
    }
    return NULL;
}

static bool apply_step(UiStepperProps* p, int dir) {
    float step = p->step > 0.0f ? p->step : 1.0f;
    float v = *p->value + (float)dir * step;

    // A zeroed min/max pair means "unbounded", which is the common case for a
    // caller that only cares about the increment.
    if (p->min != p->max) {
        if (v < p->min) v = p->min;
        if (v > p->max) v = p->max;
    }
    if (v == *p->value) return false;
    *p->value = v;
    return true;
}

UiStepperResult ui_stepper(UiStepperProps p) {
    UiStepperResult res = {0};
    if (!p.value) return res;

    const UiStyle* cs = ui_style_merge(STEPPER_CLASS_DEFAULT, p.class);

    Color fg, muted, accent, primary, input_bg, border;
    if (!ui_style_get_token("fg", &fg))            fg = (Color){1, 1, 1, 1};
    if (!ui_style_get_token("muted", &muted))      muted = fg;
    if (!ui_style_get_token("accent", &accent))    accent = (Color){0};
    if (!ui_style_get_token("primary", &primary))  primary = fg;
    if (!ui_style_get_token("input", &input_bg))   input_bg = (Color){0};
    if (!ui_style_get_token("input-border", &border)) border = muted;

    float w = p.width  > 0.0f ? p.width  : 108.0f;
    float h = p.height > 0.0f ? p.height : 24.0f;
    float btn = h;

    uint64_t key = stepper_key(p.id, p.index);
    StepperState* st = stepper_state(key);

    char dec_id[96], inc_id[96], reset_id[96];
    snprintf(dec_id, sizeof(dec_id), "%s.dec", p.id ? p.id : "stepper");
    snprintf(inc_id, sizeof(inc_id), "%s.inc", p.id ? p.id : "stepper");
    snprintf(reset_id, sizeof(reset_id), "%s.reset", p.id ? p.id : "stepper");

    // Auto-repeat, resolved before drawing so the label shows the new value on
    // the same frame the step is applied.
    if (st && !p.disabled) {
        double now = input_time_seconds();
        bool down = input_mouse_down(INPUT_MOUSE_LEFT);
        bool over_dec = uil_element_hovered(dec_id, p.index);
        bool over_inc = uil_element_hovered(inc_id, p.index);

        if (!down) {
            st->held = 0;
        } else if (st->held == 0 && input_mouse_pressed(INPUT_MOUSE_LEFT)) {
            if (over_dec) st->held = -1;
            else if (over_inc) st->held = 1;
            if (st->held != 0) {
                if (apply_step(&p, st->held)) res.changed = true;
                st->next_fire = now + STEPPER_REPEAT_DELAY;
            }
        } else if (st->held != 0 && now >= st->next_fire) {
            // Only keep repeating while the pointer stays on the button that
            // started the repeat.
            bool still_over = (st->held < 0) ? over_dec : over_inc;
            if (still_over) {
                if (apply_step(&p, st->held)) res.changed = true;
                st->next_fire = now + STEPPER_REPEAT_RATE;
            } else {
                st->held = 0;
            }
        }
    }

    char label[48];
    int decimals = p.decimals > 0 ? p.decimals : 0;
    if (p.suffix && *p.suffix) {
        snprintf(label, sizeof(label), "%.*f%s", decimals, (double)*p.value, p.suffix);
    } else {
        snprintf(label, sizeof(label), "%.*f", decimals, (double)*p.value);
    }

    UI_ROW(.gap = 6.0f, .align_y = UI_CENTER) {
        if (p.show_reset) {
            if (UI_BUTTON(.id = reset_id, .index = p.index, .icon = ICON_REFRESH,
                          .icon_size = 12.0f,
                          .w = UI_PX(20.0f), .h = UI_PX(20.0f), .pad_x = 4.0f,
                          .radius = 3.0f,
                          .fg = muted, .fg_hover = fg, .bg_hover = accent)) {
                *p.value = p.reset_value;
                res.changed = true;
                res.reset_clicked = true;
            }
        }

        UiBoxDesc box = ui_style_box(cs, NULL, 0, UI_STATE_BASE);
        box.w = UI_PX(w);
        box.h = UI_PX(h);
        box.bg = input_bg;
        box.border = 1.0f;
        box.border_color = border;
        box.align_y = UI_CENTER;

        uil__open(0 /* row */, box);

        if (UI_BUTTON(.id = dec_id, .index = p.index, .icon = ICON_CHEVRON_LEFT,
                      .icon_size = 11.0f,
                      .w = UI_PX(btn), .h = UI_GROW, .pad_x = 2.0f, .radius = 3.0f,
                      .fg = p.disabled ? color_with_alpha(muted, 0.4f) : muted,
                      .fg_hover = primary, .bg_hover = accent)) {
            // The click already fired through the repeat path above.
        }

        UI_BOX(.w = UI_GROW, .h = UI_GROW, .align_x = UI_CENTER, .align_y = UI_CENTER) {
            UI_LABEL(label, .color = p.disabled ? muted : fg, .font = UI_FONT_MONO,
                     .align = TEXT_ALIGN_CENTER);
        }

        if (UI_BUTTON(.id = inc_id, .index = p.index, .icon = ICON_CHEVRON_RIGHT,
                      .icon_size = 11.0f,
                      .w = UI_PX(btn), .h = UI_GROW, .pad_x = 2.0f, .radius = 3.0f,
                      .fg = p.disabled ? color_with_alpha(muted, 0.4f) : muted,
                      .fg_hover = primary, .bg_hover = accent)) {
        }

        uil__close();
    }

    return res;
}

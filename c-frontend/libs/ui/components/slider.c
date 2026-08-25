#include "ui/components/slider.h"
#include "input/input.h"
#include "gfx/gfx.h"

#define SLIDER_CLASS_DEFAULT "row items-center gap-2"

// A drag outlives the frame it started in, so which slider owns the pointer is
// retained rather than recomputed from hover.
typedef struct {
    uint64_t key;
    bool     dragging;
} SliderDrag;

#define SLIDER_DRAG_SLOTS 8
static SliderDrag s_drags[SLIDER_DRAG_SLOTS];

static uint64_t slider_key(const char* id, int32_t index) {
    uint64_t h = 1469598103934665603ULL;
    for (const char* p = id ? id : "slider"; *p; p++) {
        h ^= (uint8_t)*p;
        h *= 1099511628211ULL;
    }
    h ^= (uint64_t)(uint32_t)index * 1099511628211ULL;
    return h ? h : 1;
}

static SliderDrag* slider_drag(uint64_t key) {
    for (size_t i = 0; i < SLIDER_DRAG_SLOTS; i++) {
        if (s_drags[i].key == key) return &s_drags[i];
    }
    for (size_t i = 0; i < SLIDER_DRAG_SLOTS; i++) {
        if (s_drags[i].key == 0) {
            s_drags[i].key = key;
            s_drags[i].dragging = false;
            return &s_drags[i];
        }
    }
    return NULL;
}

typedef struct {
    float t;          // normalized position 0..1
    float track_h;
    Color track, fill, knob;
    float knob_r;
} SliderPaint;

#define SLIDER_MAX_PAINT 32
static SliderPaint s_paint[SLIDER_MAX_PAINT];
static size_t      s_paint_count;

// Drawn rather than composed from boxes: a track, a fill, and a knob is three
// rects, and routing them through layout would cost three elements to express
// what is already a single geometric relationship.
static void draw_slider(Rect r, void* user) {
    const SliderPaint* s = (const SliderPaint*)user;
    if (!s) return;

    float th = s->track_h;
    Rect track = rect_make(r.x, r.y + (r.h - th) * 0.5f, r.w, th);
    r_draw_rect_rounded(track, th * 0.5f, s->track);

    float fill_w = r.w * s->t;
    if (fill_w > 0.0f) {
        r_draw_rect_rounded(rect_make(track.x, track.y, fill_w, th), th * 0.5f, s->fill);
    }

    float cx = r.x + fill_w;
    float cy = r.y + r.h * 0.5f;
    r_draw_rect_rounded(rect_make(cx - s->knob_r, cy - s->knob_r,
                                  s->knob_r * 2.0f, s->knob_r * 2.0f),
                        s->knob_r, s->knob);
}

bool ui_slider(UiSliderProps p) {
    if (!p.value) return false;
    if (p.max <= p.min) return false;

    const UiStyle* cs = ui_style_merge(SLIDER_CLASS_DEFAULT, p.class);

    Color fg, muted, primary, accent;
    if (!ui_style_get_token("fg", &fg))           fg = (Color){1, 1, 1, 1};
    if (!ui_style_get_token("muted", &muted))     muted = fg;
    if (!ui_style_get_token("primary", &primary)) primary = fg;
    if (!ui_style_get_token("accent", &accent))   accent = color_with_alpha(muted, 0.30f);

    float h = p.height > 0.0f ? p.height : 16.0f;
    float th = p.track_height > 0.0f ? p.track_height : 4.0f;

    char track_id[96];
    snprintf(track_id, sizeof(track_id), "%s.track", p.id ? p.id : "slider");
    Rect track_rect = uil_element_rect(track_id, p.index);

    uint64_t key = slider_key(p.id, p.index);
    SliderDrag* drag = slider_drag(key);
    bool hovered = uil_element_hovered(track_id, p.index);
    bool changed = false;

    if (drag && !p.disabled) {
        if (hovered && input_mouse_pressed(INPUT_MOUSE_LEFT)) drag->dragging = true;
        if (!input_mouse_down(INPUT_MOUSE_LEFT)) drag->dragging = false;

        if (drag->dragging && track_rect.w > 0.0f) {
            float t = (input_mouse().x - track_rect.x) / track_rect.w;
            if (t < 0.0f) t = 0.0f;
            if (t > 1.0f) t = 1.0f;

            float v = p.min + t * (p.max - p.min);
            if (p.step > 0.0f) {
                // Snap to the step grid, measured from min so the reachable
                // values are exactly min, min+step, ... regardless of range.
                v = p.min + roundf((v - p.min) / p.step) * p.step;
            }
            if (v < p.min) v = p.min;
            if (v > p.max) v = p.max;

            if (v != *p.value) {
                *p.value = v;
                changed = true;
            }
        }
    }

    float t = (*p.value - p.min) / (p.max - p.min);
    if (t < 0.0f) t = 0.0f;
    if (t > 1.0f) t = 1.0f;

    UiBoxDesc row = ui_style_box(cs, NULL, 0, UI_STATE_BASE);
    row.w = p.w.kind ? p.w : UI_GROW;
    row.h = UI_PX(h);
    row.align_y = UI_CENTER;

    uil__open(0 /* row */, row);

    if (s_paint_count >= SLIDER_MAX_PAINT) s_paint_count = 0;
    SliderPaint* sp = &s_paint[s_paint_count++];
    sp->t = t;
    sp->track_h = th;
    sp->track = accent;
    sp->fill = p.disabled ? color_with_alpha(primary, 0.40f) : primary;
    sp->knob = p.disabled ? color_with_alpha(fg, 0.40f)
             : (hovered || (drag && drag->dragging)) ? fg : primary;
    sp->knob_r = h * 0.34f;

    // The track is its own element so its rect can be queried next frame for
    // drag maths; the paint callback fills it.
    UI_BOX(.id = track_id, .index = p.index, .w = UI_GROW, .h = UI_GROW) {
        uil_custom(UI_GROW, UI_GROW, draw_slider, sp);
    }

    if (p.show_value) {
        char buf[32];
        snprintf(buf, sizeof(buf), p.format ? p.format : "%.0f", (double)*p.value);
        UI_BOX(.w = UI_PX(44.0f), .h = UI_GROW, .align_y = UI_CENTER) {
            UI_LABEL(buf, .color = muted, .font = UI_FONT_MONO,
                     .align = TEXT_ALIGN_RIGHT);
        }
    }

    uil__close();
    return changed;
}

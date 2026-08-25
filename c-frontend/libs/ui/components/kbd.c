#include "ui/components/kbd.h"

#define KBD_CLASS_DEFAULT      "row items-center px-1 rounded-sm bg-accent"
#define KBD_TEXT_CLASS_DEFAULT "text-muted font-mono"
#define KBD_MAX_PARTS          8

// Splits a binding on '+', trimming whitespace. Returns the number of parts.
// Slices point into `binding`, so nothing is copied.
static size_t split_binding(const char* binding, const char** starts, size_t* lens,
                            size_t max) {
    if (!binding) return 0;
    size_t n = 0;
    const char* p = binding;

    while (*p && n < max) {
        while (*p == ' ' || *p == '+') p++;
        if (!*p) break;

        const char* start = p;
        while (*p && *p != '+') p++;

        const char* end = p;
        while (end > start && end[-1] == ' ') end--;   // trim trailing spaces

        if (end > start) {
            starts[n] = start;
            lens[n] = (size_t)(end - start);
            n++;
        }
    }
    return n;
}

static void emit_chip(const char* text, size_t len, const UiStyle* box_style,
                      const UiStyle* text_style, float height) {
    // The label is copied into a small stack buffer because UI_LABEL takes a
    // NUL-terminated string and the parts are slices of the caller's binding.
    char buf[32];
    size_t n = len < sizeof(buf) - 1 ? len : sizeof(buf) - 1;
    memcpy(buf, text, n);
    buf[n] = '\0';

    UiBoxDesc box = ui_style_box(box_style, NULL, 0, UI_STATE_BASE);
    box.h = UI_PX(height);
    box.w = UI_FIT;

    uil__open(0 /* row */, box);
    UiTextDesc td = ui_style_text(text_style, UI_STATE_BASE);
    uil_label(buf, td);
    uil__close();
}

void ui_kbd(const char* binding, UiKbdProps props) {
    if (!binding || !*binding) return;

    const UiStyle* bs = ui_style_merge(KBD_CLASS_DEFAULT, props.class);
    const char* text_override = props.text_class;
    const UiStyle* ts = ui_style_merge(
        props.muted ? "text-dim font-mono" : KBD_TEXT_CLASS_DEFAULT, text_override);

    float h = props.height > 0.0f ? props.height : 17.0f;

    const char* starts[KBD_MAX_PARTS];
    size_t lens[KBD_MAX_PARTS];
    size_t n = split_binding(binding, starts, lens, KBD_MAX_PARTS);

    UI_ROW(.gap = 2.0f, .align_y = UI_CENTER) {
        for (size_t i = 0; i < n; i++) {
            emit_chip(starts[i], lens[i], bs, ts, h);
        }
    }
}

void ui_kbd_list(const char* bindings, UiKbdProps props) {
    if (!bindings || !*bindings) return;

    UI_ROW(.gap = 5.0f, .align_y = UI_CENTER) {
        const char* p = bindings;
        while (*p) {
            while (*p == ' ') p++;
            if (!*p) break;

            const char* start = p;
            while (*p && *p != ' ') p++;

            char one[64];
            size_t len = (size_t)(p - start);
            size_t n = len < sizeof(one) - 1 ? len : sizeof(one) - 1;
            memcpy(one, start, n);
            one[n] = '\0';

            ui_kbd(one, props);
        }
    }
}

float ui_kbd_measure(const char* binding, UiKbdProps props) {
    if (!binding || !*binding) return 0.0f;

    const UiStyle* bs = ui_style_merge(KBD_CLASS_DEFAULT, props.class);
    const UiStyle* ts = ui_style_merge(
        props.muted ? "text-dim font-mono" : KBD_TEXT_CLASS_DEFAULT, props.text_class);

    Font* font = uil_font(ts->font);
    if (!font) return 0.0f;

    const char* starts[KBD_MAX_PARTS];
    size_t lens[KBD_MAX_PARTS];
    size_t n = split_binding(binding, starts, lens, KBD_MAX_PARTS);

    float total = 0.0f;
    for (size_t i = 0; i < n; i++) {
        total += font_measure_text(font, starts[i], lens[i]);
        total += bs->pad_l + bs->pad_r;
        if (i + 1 < n) total += 2.0f;   // inter-chip gap
    }
    return total;
}

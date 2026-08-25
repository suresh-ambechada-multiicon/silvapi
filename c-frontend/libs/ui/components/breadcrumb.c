#include "ui/components/breadcrumb.h"
#include "input/input.h"

#define CRUMB_CLASS_DEFAULT   "row items-center gap-1"
#define CRUMB_SEG_DEFAULT     "row items-center px-2 rounded-sm text-muted"
#define CRUMB_MAX             64

UiBreadcrumbResult ui_breadcrumb(const UiBreadcrumbProps* p) {
    UiBreadcrumbResult res = { .clicked = -1, .chevron_clicked = -1 };
    if (!p || !p->crumbs || p->crumb_count == 0) return res;

    const UiStyle* cs = ui_style_merge(CRUMB_CLASS_DEFAULT, p->class);
    const UiStyle* ss = ui_style_merge(CRUMB_SEG_DEFAULT, p->segment_class);

    Color muted = ui_style_fg(ss, UI_STATE_BASE);
    Color fg, accent, primary;
    if (!ui_style_get_token("fg", &fg))           fg = muted;
    if (!ui_style_get_token("accent", &accent))   accent = (Color){0};
    if (!ui_style_get_token("primary", &primary)) primary = fg;

    float h = p->height > 0.0f ? p->height : 26.0f;

    Font* font = uil_font(ss->font);
    Font* bold = uil_font(UI_FONT_BOLD);

    // Measure segments so overflow can be resolved before emitting anything.
    // The last segment is the current location and is always kept.
    float widths[CRUMB_MAX];
    size_t n = p->crumb_count < CRUMB_MAX ? p->crumb_count : CRUMB_MAX;
    float total = 0.0f;

    for (size_t i = 0; i < n; i++) {
        const UiCrumb* c = &p->crumbs[i];
        bool last = (i + 1 == n);
        Font* f = last ? bold : font;

        float w = 16.0f;                                  // segment padding
        if (c->icon != ICON_NONE) w += 14.0f + 4.0f;
        if (f && c->label) w += font_measure_text(f, c->label, 0);
        if (p->show_chevrons) w += 14.0f;                 // separator chevron
        widths[i] = w;
        total += w;
    }

    char strip_id[96];
    snprintf(strip_id, sizeof(strip_id), "%s.strip", p->id ? p->id : "crumb");
    Rect strip = uil_element_rect(strip_id, 0);
    float avail = p->max_width > 0.0f ? p->max_width
                : (strip.w > 0.0f ? strip.w : 0.0f);

    // Collapse from the left: drop leading segments until the rest fits,
    // reserving room for the ellipsis that replaces them.
    size_t first = 0;
    if (avail > 0.0f && total > avail) {
        float ellipsis_w = 28.0f;
        float fit = total;
        while (first + 1 < n && fit + ellipsis_w > avail) {
            fit -= widths[first];
            first++;
        }
        res.overflowed = true;
    }
    res.first_visible = first;

    UiBoxDesc row = ui_style_box(cs, strip_id, 0, UI_STATE_BASE);
    row.w = p->max_width > 0.0f ? UI_PX(p->max_width) : UI_GROW;
    row.h = UI_PX(h);
    row.clip_x = true;

    uil__open(0 /* row */, row);

    if (res.overflowed) {
        UI_BOX(.w = UI_PX(20.0f), .h = UI_GROW, .align_x = UI_CENTER,
               .align_y = UI_CENTER) {
            UI_LABEL("\xE2\x80\xA6", .color = muted);
        }
    }

    for (size_t i = first; i < n; i++) {
        const UiCrumb* c = &p->crumbs[i];
        bool last = (i + 1 == n);

        char seg_id[96];
        snprintf(seg_id, sizeof(seg_id), "%s.seg", p->id ? p->id : "crumb");
        bool hovered = uil_element_hovered(seg_id, (int32_t)i);

        char chev_id[96];
        snprintf(chev_id, sizeof(chev_id), "%s.chev", p->id ? p->id : "crumb");
        bool chev_hovered = p->show_chevrons && uil_element_hovered(chev_id, (int32_t)i);

        UiBoxDesc seg = ui_style_box(ss, seg_id, (int32_t)i, UI_STATE_BASE);
        seg.w = UI_FIT;
        seg.h = UI_GROW;
        seg.align_y = UI_CENTER;
        seg.gap = 4.0f;
        // The current location is highlighted; ancestors are quiet until hovered.
        seg.bg = (hovered && !chev_hovered) ? accent
               : (last ? color_with_alpha(primary, 0.14f) : (Color){0});

        uil__open(0, seg);
        if (c->icon != ICON_NONE) {
            uil_icon(c->icon, 14.0f, c->icon_color.a > 0.0f ? c->icon_color : muted);
        }
        uil_label(c->label ? c->label : "",
                  (UiTextDesc){ .color = last ? fg : (hovered ? fg : muted),
                                .font = last ? UI_FONT_BOLD : ss->font });
        uil__close();

        if (hovered && !chev_hovered && input_mouse_pressed(INPUT_MOUSE_LEFT)) {
            res.clicked = (int)i;
        }

        // Separator doubles as the sibling-list affordance, which is why it is a
        // hit target rather than decoration.
        if (p->show_chevrons && !last) {
            UI_BOX(.id = chev_id, .index = (int32_t)i,
                   .w = UI_PX(14.0f), .h = UI_GROW,
                   .align_x = UI_CENTER, .align_y = UI_CENTER,
                   .radius = 3.0f,
                   .bg = chev_hovered ? accent : (Color){0}) {
                uil_icon(ICON_CHEVRON_RIGHT, 11.0f,
                         (p->open_segment == (int)i || chev_hovered) ? fg : muted);
            }
            if (chev_hovered && input_mouse_pressed(INPUT_MOUSE_LEFT)) {
                res.chevron_clicked = (int)i;
            }
        }
    }

    if (p->editable) {
        char bg_id[96];
        snprintf(bg_id, sizeof(bg_id), "%s.bg", p->id ? p->id : "crumb");
        bool bg_hover = uil_element_hovered(bg_id, 0);
        UI_BOX(.id = bg_id, .w = UI_GROW, .h = UI_GROW) {}
        if (bg_hover && input_mouse_pressed(INPUT_MOUSE_LEFT)) {
            res.background_clicked = true;
        }
    } else {
        UI_SPACER(UI_GROW);
    }

    uil__close();
    return res;
}

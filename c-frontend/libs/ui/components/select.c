#include "ui/components/select.h"
#include "input/input.h"

#define SELECT_CLASS_DEFAULT "row items-center px-2 gap-2 rounded-sm bg-input"

bool ui_select(UiSelectProps p) {
    const UiStyle* cs = ui_style_merge(SELECT_CLASS_DEFAULT, p.class);

    Color fg, muted, accent, primary, input_bg, border, focus;
    if (!ui_style_get_token("fg", &fg))               fg = (Color){1, 1, 1, 1};
    if (!ui_style_get_token("muted", &muted))         muted = fg;
    if (!ui_style_get_token("accent", &accent))       accent = (Color){0};
    if (!ui_style_get_token("primary", &primary))     primary = fg;
    if (!ui_style_get_token("input", &input_bg))      input_bg = (Color){0};
    if (!ui_style_get_token("input-border", &border)) border = muted;
    if (!ui_style_get_token("input-focus", &focus))   focus = primary;

    float h = p.height > 0.0f ? p.height : 24.0f;
    bool hovered = p.id ? uil_element_hovered(p.id, p.index) : false;
    bool clicked = hovered && !p.disabled && input_mouse_pressed(INPUT_MOUSE_LEFT);

    bool has_value = (p.value && *p.value);
    Color label_fg = p.disabled ? color_with_alpha(muted, 0.5f)
                   : has_value  ? fg
                                : muted;

    UiBoxDesc box = ui_style_box(cs, p.id, p.index, UI_STATE_BASE);
    box.w = p.w.kind ? p.w : UI_GROW;
    box.h = UI_PX(h);
    box.bg = hovered && !p.disabled ? accent : input_bg;
    box.border = 1.0f;
    box.border_color = p.open ? focus : border;
    box.align_y = UI_CENTER;

    uil__open(0 /* row */, box);

    UI_BOX(.w = UI_GROW, .h = UI_GROW, .align_y = UI_CENTER) {
        UI_LABEL(has_value ? p.value : (p.placeholder ? p.placeholder : "Select..."),
                 .ellipsize = true, .color = label_fg);
    }

    // The chevron points down when closed and up when open, so the trigger says
    // what clicking it will do.
    uil_icon(p.open ? ICON_CHEVRON_UP : ICON_CHEVRON_DOWN, 12.0f,
             p.disabled ? color_with_alpha(muted, 0.5f) : muted);

    uil__close();
    return clicked;
}

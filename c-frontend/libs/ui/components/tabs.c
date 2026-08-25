#include "ui/components/tabs.h"
#include "input/input.h"

#define TABS_CLASS_DEFAULT      "row w-full items-center bg-tabbar"
#define TABS_TAB_DEFAULT        "row items-center px-2 gap-2 text-muted"
#define TABS_MAX_TABS           64
#define TABS_DRAG_THRESHOLD     4.0f

// Retained per-strip state: a drag spans frames, and the scroll offset must be
// known before deciding what fits.
typedef struct {
    uint64_t key;
    bool     used;

    int   drag_tab;        // -1 when not dragging
    float drag_start_x;
    float drag_dx;
    bool  drag_active;     // past the threshold, so a click is not a drag

    float scroll_x;
} TabsState;

#define TABS_STATE_SLOTS 8
static TabsState s_states[TABS_STATE_SLOTS];

static uint64_t tabs_key(const char* id) {
    uint64_t h = 1469598103934665603ULL;
    for (const char* p = id ? id : "tabs"; *p; p++) {
        h ^= (uint8_t)*p;
        h *= 1099511628211ULL;
    }
    return h ? h : 1;
}

static TabsState* tabs_state(const char* id) {
    uint64_t key = tabs_key(id);
    for (size_t i = 0; i < TABS_STATE_SLOTS; i++) {
        if (s_states[i].used && s_states[i].key == key) return &s_states[i];
    }
    for (size_t i = 0; i < TABS_STATE_SLOTS; i++) {
        if (!s_states[i].used) {
            TabsState* st = &s_states[i];
            memset(st, 0, sizeof(*st));
            st->key = key;
            st->used = true;
            st->drag_tab = -1;
            return st;
        }
    }
    return NULL;
}

UiTabsResult ui_tabs(const UiTabsProps* p) {
    UiTabsResult res = { .clicked = -1, .closed = -1, .context = -1,
                         .reorder_from = -1, .reorder_to = -1 };
    if (!p || !p->tabs || p->tab_count == 0) return res;

    TabsState* st = tabs_state(p->id);
    if (!st) return res;

    const UiStyle* ss = ui_style_merge(TABS_CLASS_DEFAULT, p->class);
    const UiStyle* ts = ui_style_merge(TABS_TAB_DEFAULT, p->tab_class);

    Color muted = ui_style_fg(ts, UI_STATE_BASE);
    Color fg, accent, primary, active_bg, inactive_bg, border;
    if (!ui_style_get_token("fg", &fg))                   fg = muted;
    if (!ui_style_get_token("accent", &accent))           accent = (Color){0};
    if (!ui_style_get_token("primary", &primary))         primary = accent;
    if (!ui_style_get_token("tab-active", &active_bg))    active_bg = accent;
    if (!ui_style_get_token("tab-inactive", &inactive_bg)) inactive_bg = (Color){0};
    if (!ui_style_get_token("border", &border))           border = muted;

    bool underline = (p->variant == UI_TABS_UNDERLINE);
    float h = p->height > 0.0f ? p->height : 32.0f;
    float min_w = p->min_tab_width > 0.0f ? p->min_tab_width : 110.0f;
    float max_w = p->max_tab_width > 0.0f ? p->max_tab_width : 220.0f;

    // Tabs are measured and clamped rather than grown. Growing them made every
    // tab fight the trailing spacer for space, so all six shrank until their
    // labels were nothing but an ellipsis.
    Font* label_font = uil_font(ts->font);
    Font* badge_font = uil_font(UI_FONT_BOLD);

    char strip_id[96];
    snprintf(strip_id, sizeof(strip_id), "%s.strip", p->id ? p->id : "tabs");
    Rect strip_rect = uil_element_rect(strip_id, 0);

    // --- drag tracking -----------------------------------------------------
    // Resolved before emitting, so the moved tab renders in its new slot on the
    // same frame the caller applies the reorder.
    if (st->drag_tab >= 0) {
        if (!input_mouse_down(INPUT_MOUSE_LEFT)) {
            st->drag_tab = -1;
            st->drag_active = false;
        } else {
            st->drag_dx = input_mouse().x - st->drag_start_x;
            if (fabsf(st->drag_dx) > TABS_DRAG_THRESHOLD) st->drag_active = true;
        }
    }

    UiBoxDesc strip = ui_style_box(ss, strip_id, 0, UI_STATE_BASE);
    strip.w = UI_GROW;
    strip.h = UI_PX(h);
    strip.clip_x = true;
    strip.gap = underline ? 2.0f : 1.0f;

    uil__open(0 /* row */, strip);

    float used_w = 0.0f;

    for (size_t i = 0; i < p->tab_count && i < TABS_MAX_TABS; i++) {
        const UiTab* tab = &p->tabs[i];
        bool active = (p->active && *p->active == (int)i);

        char tab_id[128];
        snprintf(tab_id, sizeof(tab_id), "%s.tab", p->id ? p->id : "tabs");
        bool hovered = uil_element_hovered(tab_id, (int32_t)i);

        Rect tab_rect = uil_element_rect(tab_id, (int32_t)i);
        used_w += tab_rect.w;

        Color label_fg = tab->disabled ? muted : (active ? fg : muted);
        Color tab_bg = underline ? (Color){0}
                     : active    ? active_bg
                     : hovered   ? accent
                                 : inactive_bg;

        // The tab being dragged is offset horizontally so the pointer appears to
        // carry it; layout order is unchanged until the caller applies the move.
        float drag_off = (st->drag_active && st->drag_tab == (int)i) ? st->drag_dx : 0.0f;

        // Intrinsic width: padding + badge + icon + label + trailing affordance.
        float want = 16.0f;   // horizontal padding
        if (tab->badge && *tab->badge && badge_font) {
            want += font_measure_text(badge_font, tab->badge, 0) + 6.0f;
        }
        if (tab->icon != ICON_NONE) want += 14.0f + 6.0f;
        if (label_font && tab->label) want += font_measure_text(label_font, tab->label, 0);
        if (tab->closable || tab->dirty) want += 16.0f + 6.0f;

        if (want < min_w) want = min_w;
        if (want > max_w) want = max_w;

        UI_COL(.id = tab_id, .index = (int32_t)i,
               .w = UI_PX(want),
               .h = UI_GROW, .gap = 0.0f,
               .float_offset = { drag_off, 0.0f },
               .floating = (drag_off != 0.0f),
               .float_attach = UI_ATTACH_TOP_LEFT,
               .float_z = 5,
               .bg = tab_bg,
               .radius = underline ? 0.0f : 3.0f) {

            UI_ROW(.w = UI_GROW, .h = UI_GROW, .pad_x = 8.0f, .gap = 6.0f,
                   .align_y = UI_CENTER) {

                if (tab->badge && *tab->badge) {
                    Color tint = tab->badge_color.a > 0.0f ? tab->badge_color : muted;
                    UI_LABEL(tab->badge, .color = tint, .font = UI_FONT_BOLD);
                }
                if (tab->icon != ICON_NONE) {
                    UI_ICON(tab->icon, 14.0f,
                            tab->icon_color.a > 0.0f ? tab->icon_color : label_fg);
                }

                UI_BOX(.w = UI_GROW, .h = UI_GROW, .align_y = UI_CENTER) {
                    UI_LABEL(tab->label ? tab->label : "", .ellipsize = true,
                             .color = label_fg);
                }

                // Dirty tabs show a dot that becomes the close button on hover,
                // so the close affordance never costs extra width.
                if (tab->closable) {
                    if (tab->dirty && !hovered) {
                        UI_BOX(.w = UI_PX(7.0f), .h = UI_PX(7.0f), .radius = 4.0f,
                               .bg = primary) {}
                    } else {
                        char close_id[144];
                        snprintf(close_id, sizeof(close_id), "%s.close",
                                 p->id ? p->id : "tabs");
                        if (UI_BUTTON(.id = close_id, .index = (int32_t)i,
                                      .icon = ICON_CLOSE, .icon_size = 10.0f,
                                      .w = UI_PX(16.0f), .h = UI_PX(16.0f),
                                      .pad_x = 3.0f, .radius = 3.0f,
                                      .fg = muted, .fg_hover = fg,
                                      .bg_hover = accent)) {
                            res.closed = (int)i;
                        }
                    }
                } else if (tab->dirty) {
                    UI_BOX(.w = UI_PX(7.0f), .h = UI_PX(7.0f), .radius = 4.0f,
                           .bg = primary) {}
                }
            }

            // Underline variant marks the active tab with a rule rather than a
            // fill, which is what a sub-tab row wants.
            if (underline) {
                UI_BOX(.w = UI_GROW, .h = UI_PX(2.0f),
                       .bg = active ? primary : (Color){0}) {}
            }
        }

        if (hovered && !tab->disabled) {
            char close_id[144];
            snprintf(close_id, sizeof(close_id), "%s.close", p->id ? p->id : "tabs");
            bool over_close = uil_element_hovered(close_id, (int32_t)i);

            if (input_mouse_pressed(INPUT_MOUSE_LEFT) && !over_close) {
                res.clicked = (int)i;
                if (p->active) *p->active = (int)i;
                if (p->reorderable) {
                    st->drag_tab = (int)i;
                    st->drag_start_x = input_mouse().x;
                    st->drag_dx = 0.0f;
                    st->drag_active = false;
                }
            }
            if (input_mouse_pressed(INPUT_MOUSE_RIGHT)) {
                res.context = (int)i;
            }
            if (p->close_on_middle && tab->closable &&
                input_mouse_pressed(INPUT_MOUSE_MIDDLE)) {
                res.closed = (int)i;
            }
        }

        // A drag that has moved past the midpoint of a neighbour reports a
        // reorder. Using the neighbour's midpoint rather than a fixed distance
        // keeps the behaviour correct for tabs of differing widths.
        if (st->drag_active && st->drag_tab == (int)i && tab_rect.w > 0.0f) {
            float centre = tab_rect.x + tab_rect.w * 0.5f + st->drag_dx;
            for (size_t k = 0; k < p->tab_count; k++) {
                if (k == i) continue;
                Rect other = uil_element_rect(tab_id, (int32_t)k);
                if (other.w <= 0.0f) continue;
                float o_centre = other.x + other.w * 0.5f;
                bool crossed = (k < i) ? (centre < o_centre) : (centre > o_centre);
                if (!crossed) continue;

                res.reorder_from = (int)i;
                res.reorder_to = (int)k;
                // The caller moves the tab; follow it so the drag continues from
                // its new position instead of snapping back.
                st->drag_tab = (int)k;
                st->drag_start_x = input_mouse().x;
                st->drag_dx = 0.0f;
                break;
            }
        }
    }

    if (p->show_add) {
        char add_id[128];
        snprintf(add_id, sizeof(add_id), "%s.add", p->id ? p->id : "tabs");
        if (UI_BUTTON(.id = add_id, .icon = ICON_PLUS, .icon_size = 12.0f,
                      .w = UI_PX(26.0f), .h = UI_PX(h - 8.0f), .pad_x = 5.0f,
                      .radius = 3.0f,
                      .fg = muted, .fg_hover = fg, .bg_hover = accent)) {
            res.add_clicked = true;
        }
    }

    UI_SPACER(UI_GROW);

    res.overflowing = (strip_rect.w > 0.0f && used_w > strip_rect.w);
    if (p->show_overflow && res.overflowing) {
        char of_id[128];
        snprintf(of_id, sizeof(of_id), "%s.overflow", p->id ? p->id : "tabs");
        if (UI_BUTTON(.id = of_id, .icon = ICON_CHEVRON_DOWN, .icon_size = 12.0f,
                      .w = UI_PX(24.0f), .h = UI_PX(h - 8.0f), .pad_x = 4.0f,
                      .radius = 3.0f,
                      .fg = muted, .fg_hover = fg, .bg_hover = accent)) {
            res.overflow_clicked = true;
        }
    }

    uil__close();
    return res;
}

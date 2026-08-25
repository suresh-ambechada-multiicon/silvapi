#include "ui/components/menu.h"
#include "ui/components/kbd.h"
#include "input/input.h"
#include "ui/fuzzy.h"

#define MENU_CLASS_DEFAULT    "col p-1 rounded-md bg-surface border-border"
#define MENU_ROW_DEFAULT      "row items-center px-2 gap-2 rounded-sm text-fg"
#define MENU_FILTER_DEFAULT   "row items-center px-2 h-8 gap-2"
#define MENU_MAX_ITEMS        512

int ui_menu_fuzzy_score(const char* haystack, const char* needle) {
    return fuzzy_score(haystack, needle);
}

static bool item_selectable(const UiMenuItem* it) {
    if (!it) return false;
    if (it->kind == UI_MENU_SECTION || it->kind == UI_MENU_SEPARATOR) return false;
    return !it->disabled;
}

// ---------------------------------------------------------------------------
// Filtering
// ---------------------------------------------------------------------------

typedef struct {
    int index;   // into props->items
    int score;
} Ranked;

// Builds the visible list.
//
// When filtering, structure is preserved: items are ranked WITHIN their section
// and sections keep their original order, with a heading emitted only if at
// least one of its items survived. Ranking across the whole list instead would
// scatter related commands and make the headings meaningless.
static size_t filter_items(const UiMenuProps* p, Ranked* out, size_t max) {
    const char* needle = (p->filter_buffer && p->filter_buffer[0]) ? p->filter_buffer : NULL;
    size_t n = 0;

    if (!needle) {
        for (size_t i = 0; i < p->item_count && n < max; i++) {
            out[n].index = (int)i;
            out[n].score = 0;
            n++;
        }
        return n;
    }

    size_t i = 0;
    while (i < p->item_count && n < max) {
        // A separator only earns its place between two populated groups; it is
        // decided retroactively, once we know the next group survived.
        if (p->items[i].kind == UI_MENU_SEPARATOR) {
            i++;
            continue;
        }

        int heading = -1;
        if (p->items[i].kind == UI_MENU_SECTION) {
            heading = (int)i;
            i++;
        }

        // Collect and rank this group's survivors.
        size_t group_start = n;
        while (i < p->item_count && n < max) {
            const UiMenuItem* it = &p->items[i];
            if (it->kind == UI_MENU_SECTION) break;      // next group begins
            if (it->kind == UI_MENU_SEPARATOR) { i++; continue; }

            int best = fuzzy_score(it->label, needle);
            if (it->description) {
                // A description hit counts for less than a label hit.
                int d = fuzzy_score(it->description, needle);
                if (d >= 0) d -= 6;
                if (d > best) best = d;
            }
            if (best >= 0) {
                out[n].index = (int)i;
                out[n].score = best;
                n++;
            }
            i++;
        }

        size_t group_n = n - group_start;
        if (group_n == 0) continue;    // nothing survived; drop the heading too

        // Insertion sort keeps equal scores in the author's order, which matters
        // for a curated command list.
        for (size_t a = group_start + 1; a < n; a++) {
            Ranked key = out[a];
            size_t b = a;
            while (b > group_start && out[b - 1].score < key.score) {
                out[b] = out[b - 1];
                b--;
            }
            out[b] = key;
        }

        if (heading >= 0 && n < max) {
            // Shift the group right by one and slot the heading in front of it.
            memmove(&out[group_start + 1], &out[group_start], group_n * sizeof(Ranked));
            out[group_start].index = heading;
            out[group_start].score = 0;
            n++;
        }
    }
    return n;
}

// ---------------------------------------------------------------------------
// Keyboard navigation
// ---------------------------------------------------------------------------

static void move_selection(const UiMenuProps* p, const Ranked* vis, size_t vis_n,
                           int direction) {
    if (!p->selected || vis_n == 0) return;

    // Work in visible-list space so navigation follows what is on screen.
    int cur = -1;
    for (size_t i = 0; i < vis_n; i++) {
        if (vis[i].index == *p->selected) { cur = (int)i; break; }
    }

    int step = direction;
    int idx = cur;
    for (size_t guard = 0; guard <= vis_n; guard++) {
        idx += step;
        if (idx < 0) idx = (int)vis_n - 1;
        if (idx >= (int)vis_n) idx = 0;
        if (item_selectable(&p->items[vis[idx].index])) {
            *p->selected = vis[idx].index;
            return;
        }
    }
}

// ---------------------------------------------------------------------------

UiMenuResult ui_menu(const UiMenuProps* p) {
    UiMenuResult res = { .activated = -1, .hovered = -1, .toggled = -1 };
    if (!p || !p->items || p->item_count == 0) return res;

    const UiStyle* ms = ui_style_merge(MENU_CLASS_DEFAULT, p->class);
    const UiStyle* rs = ui_style_merge(MENU_ROW_DEFAULT, p->row_class);

    Color fg      = ui_style_fg(rs, UI_STATE_BASE);
    Color muted, accent, primary, primary_fg, border;
    if (!ui_style_get_token("muted", &muted))          muted = fg;
    if (!ui_style_get_token("accent", &accent))        accent = (Color){0};
    if (!ui_style_get_token("primary", &primary))      primary = accent;
    if (!ui_style_get_token("primary-fg", &primary_fg)) primary_fg = fg;
    if (!ui_style_get_token("border", &border))        border = muted;

    // Namespaced per instance: two menus in one layout would otherwise declare
    // the same element ids and Clay would reject the duplicates, breaking
    // hit-testing on whichever came second.
    char row_id[96];
    snprintf(row_id, sizeof(row_id), "%s.row", p->id ? p->id : "menu");

    static Ranked vis[MENU_MAX_ITEMS];
    size_t vis_n = filter_items(p, vis, MENU_MAX_ITEMS);
    res.visible_count = vis_n;

    // Keyboard first, so the row emitted below already reflects the new cursor.
    if (input_key_pressed(INPUT_KEY_DOWN)) move_selection(p, vis, vis_n, +1);
    if (input_key_pressed(INPUT_KEY_UP))   move_selection(p, vis, vis_n, -1);
    if (input_key_pressed(INPUT_KEY_ESCAPE)) res.dismissed = true;

    // A filtered list whose cursor fell outside it needs the cursor pulled back
    // to the top match, otherwise Enter activates something invisible.
    if (p->selected && vis_n > 0) {
        bool cursor_ok = false;
        for (size_t i = 0; i < vis_n; i++) {
            if (vis[i].index != *p->selected) continue;
            // Visible is not enough: a heading is visible but not selectable.
            cursor_ok = item_selectable(&p->items[vis[i].index]);
            break;
        }
        if (!cursor_ok) {
            *p->selected = -1;
            for (size_t i = 0; i < vis_n; i++) {
                if (item_selectable(&p->items[vis[i].index])) {
                    *p->selected = vis[i].index;
                    break;
                }
            }
        }
    }

    float single_h = p->row_height > 0.0f ? p->row_height : 26.0f;
    float double_h = single_h + 16.0f;

    UiBoxDesc outer = ui_style_box(ms, p->id, 0, UI_STATE_BASE);
    outer.w = p->width > 0.0f ? UI_PX(p->width) : UI_FIT;
    outer.h = UI_FIT;
    outer.border_color = border;

    uil__open(1 /* col */, outer);

    // --- filter box ---
    if (p->filter_buffer && p->filter_buffer_size > 0) {
        char fid[96];
        snprintf(fid, sizeof(fid), "%s.filter", p->id ? p->id : "menu");

        UI_ROW(.w = UI_GROW, .h = UI_PX(32.0f), .pad_x = 8.0f, .gap = 6.0f,
               .align_y = UI_CENTER) {
            UI_ICON(ICON_SEARCH, 14.0f, muted);
            UI_TEXT_INPUT(.id = fid,
                          .buffer = p->filter_buffer,
                          .buffer_size = p->filter_buffer_size,
                          .placeholder = p->placeholder ? p->placeholder : "Filter...",
                          .h = UI_PX(24.0f),
                          .bg = (Color){0}, .fg = fg,
                          .placeholder_color = muted,
                          .border_color = (Color){0},
                          .border_focus_color = (Color){0},
                          .caret_color = primary,
                          .border = 0.0f);
        }
        UI_BOX(.w = UI_GROW, .h = UI_PX(1.0f), .bg = border) {}
    }

    // --- rows ---
    // A max_height turns the list into a scroll container; Clay owns the offset.
    char list_id[96];
    snprintf(list_id, sizeof(list_id), "%s.list", p->id ? p->id : "menu");

    UiBoxDesc list = {
        .id = list_id,
        .w = UI_GROW,
        .h = p->max_height > 0.0f ? UI_PX(p->max_height) : UI_FIT,
        .scroll_y = p->max_height > 0.0f,
        .clip_y = p->max_height > 0.0f,
    };

    uil__open(1, list);

    for (size_t vi = 0; vi < vis_n; vi++) {
        int idx = vis[vi].index;
        const UiMenuItem* it = &p->items[idx];

        if (it->kind == UI_MENU_SEPARATOR) {
            UI_BOX(.w = UI_GROW, .h = UI_PX(1.0f), .pad_y = 3.0f, .bg = border) {}
            continue;
        }

        if (it->kind == UI_MENU_SECTION) {
            UI_ROW(.w = UI_GROW, .h = UI_PX(single_h), .pad_x = 8.0f, .gap = 6.0f,
                   .align_y = UI_CENTER) {
                if (it->icon != ICON_NONE) UI_ICON(it->icon, 13.0f, muted);
                UI_LABEL(it->label ? it->label : "", .color = muted,
                         .font = UI_FONT_BOLD);
            }
            continue;
        }

        bool hovered  = uil_element_hovered(row_id, idx);
        bool selected = (p->selected && *p->selected == idx);
        bool two_line = (it->description && *it->description);

        if (hovered) res.hovered = idx;

        // A row hosting its own control is never flooded with the accent: any
        // control inside it would lose contrast against a solid primary fill.
        // The switch on the selected row was rendering invisible -- primary
        // track on a primary background.
        bool hosts_control = (it->kind == UI_MENU_CONTROL);
        Color row_bg = selected ? (hosts_control ? color_with_alpha(primary, 0.18f)
                                                 : primary)
                     : hovered  ? accent
                                : (Color){0};
        // With a washed background the text keeps the normal foreground, since
        // primary_fg is chosen to contrast with a solid primary fill.
        bool inverted = selected && !hosts_control;
        Color label_fg = it->disabled ? muted : (inverted ? primary_fg : fg);
        Color desc_fg  = inverted ? color_with_alpha(primary_fg, 0.75f) : muted;

        UI_ROW(.id = row_id, .index = idx,
               .w = UI_GROW, .h = UI_PX(two_line ? double_h : single_h),
               .pad_x = 8.0f, .gap = 8.0f, .align_y = UI_CENTER,
               .radius = 3.0f, .bg = row_bg) {

            if (it->icon != ICON_NONE) {
                UI_ICON(it->icon, 14.0f, label_fg);
            }

            // Label column takes the slack so trailing controls stay pinned
            // right regardless of label length.
            UI_COL(.w = UI_GROW, .h = UI_GROW, .gap = 0.0f,
                   .align_y = UI_CENTER) {
                UI_LABEL(it->label ? it->label : "", .ellipsize = true,
                         .color = label_fg);
                if (two_line) {
                    UI_LABEL(it->description, .ellipsize = true, .color = desc_fg);
                }
            }

            if (it->starred) {
                UI_ICON(ICON_SPARKLES, 13.0f, inverted ? primary_fg : muted);
            }

            if (it->shortcut && *it->shortcut) {
                ui_kbd_list(it->shortcut, (UiKbdProps){ .muted = it->disabled });
            }

            switch (it->kind) {
                case UI_MENU_CONTROL:
                    if (it->trailing) {
                        // Fixed-width when asked, so a column of controls lines
                        // up instead of each sizing to its own content.
                        UiBoxDesc slot = {
                            .w = it->trailing_width > 0.0f ? UI_PX(it->trailing_width)
                                                           : UI_FIT,
                            .h = UI_GROW,
                            .align_x = UI_END,
                            .align_y = UI_CENTER,
                        };
                        uil__open(0 /* row */, slot);
                        it->trailing(it, it->user);
                        uil__close();
                    }
                    break;

                case UI_MENU_VALUE:
                    if (it->value) {
                        UI_LABEL(it->value, .color = inverted ? primary_fg : muted);
                    }
                    UI_ICON(ICON_CHEVRON_RIGHT, 12.0f, inverted ? primary_fg : muted);
                    break;

                case UI_MENU_SUBMENU:
                    UI_ICON(ICON_CHEVRON_RIGHT, 12.0f, inverted ? primary_fg : muted);
                    break;

                case UI_MENU_CHECK:
                    if (it->checked && *it->checked) {
                        UI_ICON(ICON_CHECK, 13.0f, inverted ? primary_fg : primary);
                    } else {
                        // Reserves the same width so labels stay aligned whether
                        // or not a row is checked.
                        UI_SPACER(UI_PX(13.0f));
                    }
                    break;

                case UI_MENU_TOGGLE: {
                    bool on = it->checked && *it->checked;
                    UI_ROW(.w = UI_PX(26.0f), .h = UI_PX(14.0f), .radius = 7.0f,
                           .align_y = UI_CENTER,
                           .pad_x = 2.0f,
                           .align_x = on ? UI_END : UI_START,
                           .bg = on ? primary : color_with_alpha(muted, 0.35f)) {
                        UI_BOX(.w = UI_PX(10.0f), .h = UI_PX(10.0f), .radius = 5.0f,
                               .bg = on ? (inverted ? primary_fg : fg) : fg) {}
                    }
                    break;
                }

                default:
                    break;
            }
        }

        if (hovered && !it->disabled && input_mouse_pressed(INPUT_MOUSE_LEFT)) {
            if (p->selected) *p->selected = idx;
            // A row hosting its own control reports selection but not
            // activation: the click belongs to the slider or stepper inside it.
            if (it->kind == UI_MENU_CONTROL) {
                res.hovered = idx;
                continue;
            }
            if (it->kind == UI_MENU_TOGGLE && it->checked) {
                *it->checked = !*it->checked;
                res.toggled = idx;
            }
            res.activated = idx;
        }
    }

    uil__close();   // list

    // --- footer hints ---
    if (p->show_hints) {
        UI_BOX(.w = UI_GROW, .h = UI_PX(1.0f), .bg = border) {}
        UI_ROW(.w = UI_GROW, .h = UI_PX(26.0f), .gap = 6.0f,
               .align_x = UI_CENTER, .align_y = UI_CENTER) {
            ui_kbd("Up", (UiKbdProps){ .muted = true });
            ui_kbd("Down", (UiKbdProps){ .muted = true });
            UI_LABEL("to navigate", .color = muted);
            ui_kbd("Enter", (UiKbdProps){ .muted = true });
            UI_LABEL("to use", .color = muted);
            ui_kbd("Escape", (UiKbdProps){ .muted = true });
            UI_LABEL("to dismiss", .color = muted);
        }
    }

    uil__close();   // outer

    // Enter activates after the rows are emitted, so `selected` reflects any
    // arrow keys pressed this same frame.
    if (p->selected && *p->selected >= 0 && input_key_pressed(INPUT_KEY_ENTER)) {
        const UiMenuItem* it = &p->items[*p->selected];
        if (item_selectable(it)) {
            if (it->kind == UI_MENU_TOGGLE && it->checked) {
                *it->checked = !*it->checked;
                res.toggled = *p->selected;
            }
            res.activated = *p->selected;
        }
    }

    return res;
}

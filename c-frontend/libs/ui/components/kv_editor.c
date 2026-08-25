#include "ui/components/kv_editor.h"
#include "input/input.h"

#define KV_CLASS_DEFAULT "col w-full h-full gap-1"
#define KV_ROW_DEFAULT   "row items-center gap-2 text-fg"
#define KV_CHECK_W       22.0f
#define KV_REMOVE_W      22.0f

// Not virtualized: a key/value list is bounded by what a person types into it,
// and every row owns focusable text inputs whose retained state is keyed per
// row. If a generated collection ever produces thousands of headers, this is the
// place to adopt the culling the tree and table use.
#define KV_MAX_ROWS 512

UiKvEditorResult ui_kv_editor(const UiKvEditorProps* p) {
    UiKvEditorResult res = { .removed = -1, .toggled = -1 };
    if (!p || !p->row) return res;

    const UiStyle* cs = ui_style_merge(KV_CLASS_DEFAULT, p->class);
    const UiStyle* rs = ui_style_merge(KV_ROW_DEFAULT, p->row_class);

    Color fg = ui_style_fg(rs, UI_STATE_BASE);
    Color muted, accent, primary, input_bg, input_border, input_focus, placeholder, danger;
    if (!ui_style_get_token("muted", &muted))               muted = fg;
    if (!ui_style_get_token("accent", &accent))             accent = (Color){0};
    if (!ui_style_get_token("primary", &primary))           primary = fg;
    if (!ui_style_get_token("input", &input_bg))            input_bg = (Color){0};
    if (!ui_style_get_token("input-border", &input_border)) input_border = muted;
    if (!ui_style_get_token("input-focus", &input_focus))   input_focus = primary;
    if (!ui_style_get_token("placeholder", &placeholder))   placeholder = muted;
    if (!ui_style_get_token("danger", &danger))             danger = muted;

    float row_h = p->row_height > 0.0f ? p->row_height : 30.0f;
    float key_frac = (p->key_fraction > 0.05f && p->key_fraction < 0.95f)
                        ? p->key_fraction : 0.40f;
    // With a description column the key and description share what the value
    // does not take, so the value column stays the widest of the three.
    float desc_frac = p->show_description ? key_frac * 0.6f : 0.0f;
    float key_col = p->show_description ? key_frac * 0.7f : key_frac;

    const char* kph = p->key_placeholder   ? p->key_placeholder   : "Key";
    const char* vph = p->value_placeholder ? p->value_placeholder : "Value";

    size_t n = p->row_count < KV_MAX_ROWS ? p->row_count : KV_MAX_ROWS;

    UiBoxDesc outer = ui_style_box(cs, p->id, 0, UI_STATE_BASE);
    outer.w = UI_GROW;
    outer.h = p->max_height > 0.0f ? UI_PX(p->max_height) : UI_GROW;

    uil__open(1 /* col */, outer);

    // --- header ---
    // Column widths are shared with the rows below by construction: the same
    // fractions and the same fixed gutters, so a header can never drift out of
    // alignment with its column.
    if (p->show_header) {
        UI_ROW(.w = UI_GROW, .h = UI_PX(row_h - 4.0f), .gap = 8.0f,
               .align_y = UI_CENTER) {
            UI_SPACER(UI_PX(KV_CHECK_W));
            UI_BOX(.w = UI_PCT(key_col), .h = UI_GROW, .align_y = UI_CENTER) {
                UI_LABEL(kph, .color = fg, .font = UI_FONT_BOLD);
            }
            UI_BOX(.w = UI_GROW, .h = UI_GROW, .align_y = UI_CENTER) {
                UI_LABEL(vph, .color = fg, .font = UI_FONT_BOLD);
            }
            if (p->show_description) {
                UI_BOX(.w = UI_PCT(desc_frac), .h = UI_GROW, .align_y = UI_CENTER) {
                    UI_LABEL("Description", .color = fg, .font = UI_FONT_BOLD);
                }
            }
            if (p->allow_remove) UI_SPACER(UI_PX(KV_REMOVE_W));
        }
    }

    // --- rows ---
    char list_id[96];
    snprintf(list_id, sizeof(list_id), "%s.list", p->id ? p->id : "kv");

    UiBoxDesc list = {
        .id = list_id,
        .w = UI_GROW,
        .h = UI_GROW,
        .gap = 3.0f,
        .scroll_y = true,
        .clip_y = true,
    };
    uil__open(1, list);

    for (size_t i = 0; i < n; i++) {
        UiKvRow row = {0};
        p->row(i, &row, p->user);

        UI_ROW(.w = UI_GROW, .h = UI_PX(row_h), .gap = 8.0f, .align_y = UI_CENTER) {

            if (row.enabled) {
                char cb[96];
                snprintf(cb, sizeof(cb), "%s.on", p->id ? p->id : "kv");
                if (UI_CHECKBOX(.id = cb, .index = (int32_t)i, .value = row.enabled,
                                .size = 16.0f, .radius = 3.0f,
                                .bg = input_bg, .border_color = input_border,
                                .check_color = primary)) {
                    res.toggled = (int)i;
                }
            } else {
                UI_SPACER(UI_PX(KV_CHECK_W));
            }

            char kid[96], vid[96], did[96];
            snprintf(kid, sizeof(kid), "%s.k", p->id ? p->id : "kv");
            snprintf(vid, sizeof(vid), "%s.v", p->id ? p->id : "kv");
            snprintf(did, sizeof(did), "%s.d", p->id ? p->id : "kv");

            UI_BOX(.w = UI_PCT(key_col), .h = UI_GROW, .align_y = UI_CENTER) {
                if (row.key && row.key_cap > 0 &&
                    UI_TEXT_INPUT(.id = kid, .index = (int32_t)i,
                                  .buffer = row.key, .buffer_size = row.key_cap,
                                  .placeholder = kph, .readonly = row.readonly,
                                  .bg = input_bg, .fg = fg,
                                  .placeholder_color = placeholder,
                                  .border_color = input_border,
                                  .border_focus_color = input_focus,
                                  .caret_color = primary, .radius = 3.0f)) {
                    res.changed = true;
                }
            }

            UI_BOX(.w = UI_GROW, .h = UI_GROW, .align_y = UI_CENTER) {
                if (row.value && row.value_cap > 0 &&
                    UI_TEXT_INPUT(.id = vid, .index = (int32_t)i,
                                  .buffer = row.value, .buffer_size = row.value_cap,
                                  .placeholder = vph, .readonly = row.readonly,
                                  .bg = input_bg, .fg = fg,
                                  .placeholder_color = placeholder,
                                  .border_color = input_border,
                                  .border_focus_color = input_focus,
                                  .caret_color = primary, .radius = 3.0f)) {
                    res.changed = true;
                }
            }

            if (p->show_description) {
                UI_BOX(.w = UI_PCT(desc_frac), .h = UI_GROW, .align_y = UI_CENTER) {
                    if (row.description && row.description_cap > 0 &&
                        UI_TEXT_INPUT(.id = did, .index = (int32_t)i,
                                      .buffer = row.description,
                                      .buffer_size = row.description_cap,
                                      .placeholder = "Description",
                                      .readonly = row.readonly,
                                      .bg = input_bg, .fg = muted,
                                      .placeholder_color = placeholder,
                                      .border_color = input_border,
                                      .border_focus_color = input_focus,
                                      .caret_color = primary, .radius = 3.0f)) {
                        res.changed = true;
                    }
                }
            }

            if (p->allow_remove) {
                char rid[96];
                snprintf(rid, sizeof(rid), "%s.rm", p->id ? p->id : "kv");
                if (UI_BUTTON(.id = rid, .index = (int32_t)i, .icon = ICON_CLOSE,
                              .icon_size = 11.0f,
                              .w = UI_PX(KV_REMOVE_W), .h = UI_PX(20.0f),
                              .pad_x = 4.0f, .radius = 3.0f,
                              .fg = muted, .fg_hover = danger,
                              .bg_hover = accent)) {
                    res.removed = (int)i;
                }
            }
        }
    }

    // Last row of the scrollable list, so it always sits directly under the
    // rows and scrolls with them.

    if (p->allow_add) {
        char add_id[96];
        snprintf(add_id, sizeof(add_id), "%s.add", p->id ? p->id : "kv");
        UI_ROW(.w = UI_GROW, .h = UI_PX(row_h - 4.0f), .align_y = UI_CENTER) {
            if (UI_BUTTON(.id = add_id, .icon = ICON_PLUS,
                          .label = p->add_label ? p->add_label : "Add",
                          .icon_size = 12.0f, .h = UI_PX(22.0f), .radius = 3.0f,
                          .fg = primary, .fg_hover = fg, .bg_hover = accent)) {
                res.add_clicked = true;
            }
        }
    }

    uil__close();   // list

    uil__close();   // outer
    return res;
}

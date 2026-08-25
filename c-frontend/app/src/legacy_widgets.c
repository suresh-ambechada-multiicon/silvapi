#include "app/legacy_widgets.h"
#include "gfx/gfx.h"

// ---------------------------------------------------------------------------
// The immediate-mode key/value editor, relocated out of libs/ui.
//
// It takes the application's KeyValue type directly, which is what forced the
// whole widget library to include app/model.h. Moving it here rather than
// wrapping it removes that dependency outright: it is application code that
// happens to draw, exactly like the old http_client was application code that
// happened to do networking.
//
// Its replacement is libs/ui/components/kv_editor.c, which reaches rows through
// an accessor and therefore knows nothing about KeyValue. This copy exists only
// until request_view.c's body editor is ported, since the callers that remain
// are inside an immediate-mode draw and cannot emit declarative layout.
// ---------------------------------------------------------------------------

bool ui_kv_editor_legacy(const char* id_str, KeyValue** items_ptr, size_t* count_ptr, Rect rect, float* scroll_y) {
    Theme* t = theme_get_active();
    if (!items_ptr || !count_ptr) return false;

    float row_h = 28.0f;
    float header_h = 26.0f;
    size_t count = *count_ptr;
    KeyValue* items = *items_ptr;

    uint64_t id = ui_hash_id(id_str ? id_str : "kv_editor", 0);
    float total_content_h = header_h + count * (row_h + 2.0f) + 40.0f;
    float current_scroll = ui_smooth_scroll(id, rect, total_content_h, scroll_y);

    // Header row
    float h_fh = font_get_height(g_ui.font_bold);
    r_draw_rect(rect_make(rect.x, rect.y, rect.w, header_h), t->panel_bg);
    font_draw_text(g_ui.font_bold, "Key", 0, rect.x + 36.0f, rect.y + (header_h - h_fh) * 0.5f, t->text_muted);
    font_draw_text(g_ui.font_bold, "Value", 0, rect.x + rect.w * 0.45f + 8.0f, rect.y + (header_h - h_fh) * 0.5f, t->text_muted);

    r_push_clip(rect_make(rect.x, rect.y + header_h, rect.w, rect.h - header_h));
    float cur_y = rect.y + header_h + 2.0f - current_scroll;

    bool changed = false;
    for (size_t i = 0; i < count; i++) {
        Rect row_rect = rect_make(rect.x, cur_y, rect.w, row_h);
        if (cur_y + row_h >= rect.y && cur_y <= rect.y + rect.h) {
            // Checkbox
            Rect cb_rect = rect_make(row_rect.x + 4.0f, row_rect.y, 24.0f, row_h);
            if (ui_checkbox(NULL, &items[i].enabled, cb_rect)) changed = true;

            // Key Input
            float key_w = rect.w * 0.4f;
            Rect key_rect = rect_make(row_rect.x + 32.0f, row_rect.y + 2.0f, key_w, row_h - 4.0f);
            char key_id[64];
            snprintf(key_id, sizeof(key_id), "%s_k_%zu", id_str, i);
            if (ui_input_text(key_id, items[i].key, sizeof(items[i].key), "Key", key_rect)) changed = true;

            // Value Input
            float val_w = rect.w - key_w - 74.0f;
            Rect val_rect = rect_make(key_rect.x + key_w + 6.0f, row_rect.y + 2.0f, val_w, row_h - 4.0f);
            char val_id[64];
            snprintf(val_id, sizeof(val_id), "%s_v_%zu", id_str, i);
            if (ui_input_text(val_id, items[i].value, sizeof(items[i].value), "Value", val_rect)) changed = true;

            // Delete Row '×'
            Rect del_rect = rect_make(val_rect.x + val_w + 6.0f, row_rect.y + 4.0f, 20.0f, 20.0f);
            if (ui_button_ghost("×", del_rect)) {
                kv_array_remove(items_ptr, count_ptr, i);
                changed = true;
                break;
            }
        }

        cur_y += row_h + 2.0f;
    }

    // '+ Add' Button
    Rect add_rect = rect_make(rect.x + 4.0f, cur_y + 4.0f, 64.0f, 24.0f);
    if (ui_button_ghost("+ Add", add_rect)) {
        kv_array_add(items_ptr, count_ptr, "", "", true);
        changed = true;
    }

    r_pop_clip();
    ui_draw_scrollbar(id, rect, total_content_h);
    return changed;
}

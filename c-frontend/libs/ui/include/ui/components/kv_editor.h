#ifndef UI_COMPONENTS_KV_EDITOR_H
#define UI_COMPONENTS_KV_EDITOR_H

#include "ui/layout.h"
#include "ui/style.h"

// ---------------------------------------------------------------------------
// Key/value editor: the headers, query params, and environment-variable table.
//
// Rows are reached through an accessor rather than copied in, because the text
// inputs write DIRECTLY into the caller's buffers. Copying rows into a component
// -owned array each frame would need copying back out again, which is both
// wasteful and a second source of truth for the value being typed.
//
//     static void row_at(size_t i, UiKvRow* out, void* user) {
//         KeyValue* kv = &((Workspace*)user)->variables[i];
//         out->key       = kv->key;    out->key_cap   = sizeof(kv->key);
//         out->value     = kv->value;  out->value_cap = sizeof(kv->value);
//         out->enabled   = &kv->enabled;
//         out->id        = kv->id;
//     }
//
// The accessor is the whole point of this header: it is what lets the widget
// library stop depending on the application's model. The function this replaces
// took a `KeyValue**` straight from the app, which is why libs/ui had to include
// app/model.h at all.
// ---------------------------------------------------------------------------

typedef struct UiKvRow {
    const char* id;        // stable row identity; falls back to the index

    char*  key;
    size_t key_cap;
    char*  value;
    size_t value_cap;

    // Optional third column; NULL hides it for this row.
    char*  description;
    size_t description_cap;

    bool*  enabled;        // NULL hides the checkbox for this row
    bool   readonly;
} UiKvRow;

typedef void (*UiKvRowFn)(size_t index, UiKvRow* out, void* user);

typedef struct {
    const char* id;
    const char* class;
    const char* row_class;

    size_t     row_count;
    UiKvRowFn  row;
    void*      user;

    const char* key_placeholder;    // default "Key"
    const char* value_placeholder;  // default "Value"
    const char* add_label;          // default "Add"

    // Share of the row width given to the key column, 0..1. Default 0.4.
    float key_fraction;

    bool show_header;
    bool show_description;
    bool allow_add;
    bool allow_remove;

    float row_height;   // 0 = derived
    float max_height;   // 0 = fill parent; beyond this the list scrolls
} UiKvEditorProps;

typedef struct {
    bool changed;       // a key, value, or description was edited this frame
    int  removed;       // index the user asked to remove, -1
    int  toggled;       // index whose enabled flag flipped, -1
    bool add_clicked;
} UiKvEditorResult;

UiKvEditorResult ui_kv_editor(const UiKvEditorProps* props);

#endif // UI_COMPONENTS_KV_EDITOR_H

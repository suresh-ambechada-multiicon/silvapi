#ifndef UI_COMPONENTS_TABLE_H
#define UI_COMPONENTS_TABLE_H

#include "ui/layout.h"
#include "ui/style.h"

// ---------------------------------------------------------------------------
// Data table.
//
// Virtualized: only the rows intersecting the viewport are emitted, so a
// million-row table costs the same per frame as a twenty-row one. Cells are
// pulled through a callback rather than pushed as an array, so the caller never
// materializes rows it cannot see.
//
// Column widths, sort state, order, and visibility live in the caller's
// UiTableColumn array. The table mutates that array in place (a resize drag
// writes `width`), so persisting layout is just persisting the array.
//
//     static UiTableColumn cols[] = {
//         { .id = "name", .title = "Name", .width = 240, .resizable = true,
//           .sortable = true, .visible = true, .grow = true },
//         { .id = "size", .title = "Size", .width = 90, .align = TEXT_ALIGN_RIGHT,
//           .resizable = true, .sortable = true, .visible = true },
//     };
//
//     UiTableResult r = ui_table(&(UiTableProps){
//         .id = "files", .columns = cols, .column_count = 2,
//         .row_count = n, .cell = fill_cell, .user = model,
//         .selected_row = &sel, .sort_column = &sort_col, .sort_desc = &desc,
//     });
//     if (r.activated_row >= 0) open(r.activated_row);
// ---------------------------------------------------------------------------

typedef enum {
    UI_CELL_TEXT = 0,
    UI_CELL_BADGE,      // pill with a tinted background, sized to its text
    UI_CELL_PROGRESS,   // bar plus a percentage label
    UI_CELL_ICON_TEXT,
    UI_CELL_CUSTOM,
} UiCellKind;

typedef struct {
    UiCellKind  kind;
    const char* text;      // must stay valid until end of frame
    Color       color;     // badge tint / progress fill / text override; a=0 uses the default
    float       value;     // UI_CELL_PROGRESS: 0..1
    IconType    icon;
    UiDrawFn    draw;      // UI_CELL_CUSTOM
    void*       user;
    bool        dim;       // renders muted, for "--" style empty values
} UiCell;

typedef struct {
    const char* id;         // stable; used for width persistence and hit-testing
    const char* title;
    float       width;      // current width in px; mutated by a resize drag
    float       min_width;  // default 40
    bool        resizable;
    bool        sortable;
    bool        visible;
    bool        grow;       // absorbs leftover width
    TextAlign   align;
} UiTableColumn;

// Fills `out` for one cell. Called only for cells that are actually visible.
typedef void (*UiTableCellFn)(size_t row, size_t col, UiCell* out, void* user);

typedef struct {
    const char* id;

    // Style overrides, merged over the component defaults.
    const char* class;
    const char* header_class;
    const char* row_class;

    UiTableColumn* columns;
    size_t         column_count;

    size_t        row_count;
    float         row_height;    // 0 = derived from the font
    UiTableCellFn cell;
    void*         user;

    bool show_header;            // default true unless hide_header is set
    bool hide_header;
    bool striped;
    bool multi_select;
    bool row_dividers;

    // Caller-owned state. NULL disables the corresponding behaviour.
    int*  selected_row;   // -1 for none
    int*  sort_column;    // index into columns, -1 for none
    bool* sort_desc;
} UiTableProps;

typedef struct {
    int  clicked_row;         // -1 none
    int  activated_row;       // double-clicked
    int  context_row;         // right-clicked
    int  context_column;      // right-clicked header column, -1 none
    bool sort_changed;
    bool columns_changed;     // a width changed this frame

    // Visible window, useful for a status bar or for prefetching.
    size_t first_visible_row;
    size_t visible_row_count;
} UiTableResult;

UiTableResult ui_table(const UiTableProps* props);

// Sets every column's width to fit its title. Real content-fitting needs the
// caller's data, so it takes a measuring callback.
void ui_table_autosize_columns(UiTableColumn* cols, size_t count,
                              UiTableCellFn cell, void* user, size_t sample_rows);

#endif // UI_COMPONENTS_TABLE_H

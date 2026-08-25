#include "ui/components/table.h"
#include "input/input.h"
#include "gfx/gfx.h"

// ---------------------------------------------------------------------------
// Style defaults. A caller's `class` is merged over these, so overriding one
// property does not mean restating all of them.
// ---------------------------------------------------------------------------

#define TABLE_CLASS_DEFAULT  "col w-full h-full bg-surface"
#define TABLE_HEADER_DEFAULT "row w-full items-center bg-surface text-muted font-bold"
#define TABLE_ROW_DEFAULT    "row w-full items-center text-fg"

#define TABLE_RESIZE_GRIP    5.0f    // px either side of a column edge
#define TABLE_MAX_COLUMNS    32

// Retained per-table state. Scroll lives here rather than in Clay because
// virtualization needs the offset *before* deciding which rows to emit, and
// Clay's own offset is only readable after the fact.
typedef struct {
    uint64_t key;
    float    scroll_y;
    float    scroll_x;
    int      resizing_col;     // -1 when not dragging
    float    resize_start_w;
    float    resize_start_x;
    int      anchor_row;       // for shift-range selection
    double   last_click_time;
    int      last_click_row;
    bool     used;
} TableState;

#define TABLE_STATE_SLOTS 32
static TableState s_states[TABLE_STATE_SLOTS];

static uint64_t table_key(const char* id) {
    uint64_t h = 1469598103934665603ULL;
    for (const char* p = id ? id : "table"; *p; p++) {
        h ^= (uint8_t)*p;
        h *= 1099511628211ULL;
    }
    return h ? h : 1;
}

static TableState* table_state(const char* id) {
    uint64_t key = table_key(id);
    for (size_t i = 0; i < TABLE_STATE_SLOTS; i++) {
        if (s_states[i].used && s_states[i].key == key) return &s_states[i];
    }
    for (size_t i = 0; i < TABLE_STATE_SLOTS; i++) {
        if (!s_states[i].used) {
            TableState* st = &s_states[i];
            memset(st, 0, sizeof(*st));
            st->key = key;
            st->used = true;
            st->resizing_col = -1;
            st->anchor_row = -1;
            st->last_click_row = -1;
            return st;
        }
    }
    return NULL;
}

// ---------------------------------------------------------------------------
// Column geometry
// ---------------------------------------------------------------------------

typedef struct {
    int   index;      // index into props->columns
    float width;
    float x;          // left edge, relative to the table's content area
} ColLayout;

// Resolves visible columns into concrete widths. Growing columns share whatever
// is left after the fixed ones, which is what keeps a "Name" column absorbing
// the window while "Size" stays put.
static size_t layout_columns(const UiTableProps* p, float avail,
                             ColLayout* out, size_t max, float* out_total) {
    size_t n = 0;
    float fixed = 0.0f;
    size_t grow_count = 0;

    for (size_t i = 0; i < p->column_count && n < max; i++) {
        const UiTableColumn* c = &p->columns[i];
        if (!c->visible) continue;

        out[n].index = (int)i;
        out[n].width = c->width > 0.0f ? c->width : 120.0f;
        if (c->grow) grow_count++;
        else         fixed += out[n].width;
        n++;
    }

    if (grow_count > 0) {
        float leftover = avail - fixed;
        for (size_t i = 0; i < n; i++) {
            const UiTableColumn* c = &p->columns[out[i].index];
            if (!c->grow) continue;
            float share = leftover / (float)grow_count;
            float min_w = c->min_width > 0.0f ? c->min_width : 40.0f;
            out[i].width = share > min_w ? share : min_w;
        }
    }

    float x = 0.0f;
    float total = 0.0f;
    for (size_t i = 0; i < n; i++) {
        out[i].x = x;
        x += out[i].width;
        total += out[i].width;
    }
    if (out_total) *out_total = total;
    return n;
}

// ---------------------------------------------------------------------------
// Cell rendering
// ---------------------------------------------------------------------------

// Progress cells are drawn rather than composed from boxes: a bar is one rect
// plus one fill, and routing it through layout would cost two elements per row
// for no benefit.
typedef struct {
    float value;
    Color fill;
    Color track;
} ProgressPayload;

#define TABLE_MAX_PROGRESS 512
static ProgressPayload s_progress[TABLE_MAX_PROGRESS];
static size_t          s_progress_count;

static void draw_progress(Rect r, void* user) {
    const ProgressPayload* p = (const ProgressPayload*)user;
    if (!p) return;

    float h = r.h > 14.0f ? 14.0f : r.h;
    Rect track = rect_make(r.x, r.y + (r.h - h) * 0.5f, r.w, h);
    r_draw_rect_rounded(track, 3.0f, p->track);

    float v = p->value < 0.0f ? 0.0f : (p->value > 1.0f ? 1.0f : p->value);
    if (v > 0.0f) {
        Rect fill = rect_make(track.x, track.y, track.w * v, track.h);
        r_draw_rect_rounded(fill, 3.0f, p->fill);
    }
}

static void emit_cell(const UiCell* cell, const UiTableColumn* col,
                      float width, float row_h, const UiStyle* row_style,
                      size_t row, size_t col_idx) {
    Color fg = ui_style_fg(row_style, UI_STATE_BASE);
    Color muted;
    if (!ui_style_get_token("muted", &muted)) muted = fg;

    UiBoxDesc cellbox = {
        .w = UI_PX(width), .h = UI_PX(row_h),
        .pad_x = 8.0f, .gap = 6.0f,
        .align_y = UI_CENTER,
        .align_x = col->align == TEXT_ALIGN_RIGHT  ? UI_END
                 : col->align == TEXT_ALIGN_CENTER ? UI_CENTER : UI_START,
        .clip_x = true,
    };

    uil__open(0 /* row */, cellbox);

    switch (cell->kind) {
        case UI_CELL_BADGE: {
            Color tint = cell->color.a > 0.0f ? cell->color : fg;
            UI_ROW(.h = UI_PX(18.0f), .pad_x = 5.0f, .align_y = UI_CENTER,
                   .radius = 3.0f, .bg = color_with_alpha(tint, 0.16f)) {
                UI_LABEL(cell->text ? cell->text : "", .color = tint,
                         .font = UI_FONT_BOLD);
            }
            break;
        }

        case UI_CELL_PROGRESS: {
            if (s_progress_count < TABLE_MAX_PROGRESS) {
                ProgressPayload* pp = &s_progress[s_progress_count++];
                pp->value = cell->value;
                pp->fill  = cell->color.a > 0.0f ? cell->color : fg;
                Color track;
                if (!ui_style_get_token("accent", &track)) track = color_with_alpha(fg, 0.15f);
                pp->track = track;
                uil_custom(UI_GROW, UI_GROW, draw_progress, pp);
            }
            if (cell->text && *cell->text) {
                UI_LABEL(cell->text, .color = muted);
            }
            break;
        }

        case UI_CELL_ICON_TEXT: {
            if (cell->icon != ICON_NONE) {
                UI_ICON(cell->icon, 14.0f, cell->color.a > 0.0f ? cell->color : fg);
            }
            UI_LABEL(cell->text ? cell->text : "", .ellipsize = true,
                     .align = col->align,
                     .color = cell->dim ? muted
                            : (cell->color.a > 0.0f ? cell->color : fg));
            break;
        }

        case UI_CELL_CUSTOM:
            if (cell->draw) uil_custom(UI_GROW, UI_GROW, cell->draw, cell->user);
            break;

        case UI_CELL_TEXT:
        default:
            UI_LABEL(cell->text ? cell->text : "", .ellipsize = true,
                     .align = col->align,
                     .color = cell->dim ? muted
                            : (cell->color.a > 0.0f ? cell->color : fg));
            break;
    }

    uil__close();
    (void)row;
    (void)col_idx;
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

static void emit_header(const UiTableProps* p, const UiStyle* hs,
                        const ColLayout* cols, size_t col_count,
                        float row_h, TableState* st, UiTableResult* res) {
    UiBoxDesc hdr = ui_style_box(hs, NULL, 0, UI_STATE_BASE);
    hdr.w = UI_GROW;
    hdr.h = UI_PX(row_h);
    hdr.clip_x = true;

    Color muted = ui_style_fg(hs, UI_STATE_BASE);
    Color fg;
    if (!ui_style_get_token("fg", &fg)) fg = muted;
    Color divider;
    if (!ui_style_get_token("border", &divider)) divider = muted;

    uil__open(0, hdr);

    for (size_t i = 0; i < col_count; i++) {
        const UiTableColumn* c = &p->columns[cols[i].index];

        char cell_id[96];
        snprintf(cell_id, sizeof(cell_id), "%s.hdr.%s", p->id ? p->id : "tbl", c->id);
        bool hovered = uil_element_hovered(cell_id, 0);
        bool is_sorted = (p->sort_column && *p->sort_column == cols[i].index);

        UI_ROW(.id = cell_id, .w = UI_PX(cols[i].width), .h = UI_GROW,
               .pad_x = 8.0f, .gap = 4.0f, .align_y = UI_CENTER,
               .bg = hovered ? color_with_alpha(fg, 0.06f) : (Color){0}) {

            UI_BOX(.w = UI_GROW, .h = UI_GROW, .align_y = UI_CENTER) {
                UI_LABEL(c->title ? c->title : c->id, .ellipsize = true,
                         .color = is_sorted ? fg : muted, .font = UI_FONT_BOLD);
            }

            // Sort arrow only on the sorted column, so the header stays quiet.
            if (is_sorted) {
                bool desc = p->sort_desc && *p->sort_desc;
                UI_ICON(desc ? ICON_CHEVRON_DOWN : ICON_CHEVRON_UP, 12.0f, fg);
            }
        }

        // Divider between columns, drawn as a layout box so it cannot drift out
        // of alignment with the column edge.
        if (i + 1 < col_count) {
            UI_BOX(.w = UI_PX(1.0f), .h = UI_PCT(0.55f), .bg = divider) {}
        }

        if (hovered && c->sortable && input_mouse_pressed(INPUT_MOUSE_LEFT) &&
            st->resizing_col < 0) {
            if (p->sort_column) {
                if (*p->sort_column == cols[i].index) {
                    if (p->sort_desc) *p->sort_desc = !*p->sort_desc;
                } else {
                    *p->sort_column = cols[i].index;
                    if (p->sort_desc) *p->sort_desc = false;
                }
                res->sort_changed = true;
            }
        }
        if (hovered && input_mouse_pressed(INPUT_MOUSE_RIGHT)) {
            res->context_column = cols[i].index;
        }
    }

    uil__close();
}

// Column resize is hit-tested against last frame's header rects rather than
// through layout: the grip straddles a column boundary, so it belongs to
// neither element.
static void handle_column_resize(const UiTableProps* p, const ColLayout* cols,
                                 size_t col_count, TableState* st,
                                 Rect header_rect, UiTableResult* res) {
    Vec2 m = input_mouse();

    if (st->resizing_col >= 0) {
        if (!input_mouse_down(INPUT_MOUSE_LEFT)) {
            st->resizing_col = -1;
        } else {
            UiTableColumn* c = &p->columns[st->resizing_col];
            float min_w = c->min_width > 0.0f ? c->min_width : 40.0f;
            float w = st->resize_start_w + (m.x - st->resize_start_x);
            c->width = w < min_w ? min_w : w;
            // A resized column stops absorbing leftover space, otherwise the
            // drag fights the grow calculation and the width snaps back.
            c->grow = false;
            res->columns_changed = true;
            return;
        }
    }

    if (header_rect.h <= 0.0f) return;
    bool over_header = (m.y >= header_rect.y && m.y <= header_rect.y + header_rect.h);
    if (!over_header) return;

    for (size_t i = 0; i < col_count; i++) {
        const UiTableColumn* c = &p->columns[cols[i].index];
        if (!c->resizable) continue;

        float edge = header_rect.x + cols[i].x + cols[i].width - st->scroll_x;
        if (fabsf(m.x - edge) > TABLE_RESIZE_GRIP) continue;

        if (input_mouse_pressed(INPUT_MOUSE_LEFT)) {
            st->resizing_col = cols[i].index;
            st->resize_start_w = cols[i].width;
            st->resize_start_x = m.x;
        }
        break;
    }
}

// ---------------------------------------------------------------------------

UiTableResult ui_table(const UiTableProps* p) {
    UiTableResult res = {
        .clicked_row = -1, .activated_row = -1,
        .context_row = -1, .context_column = -1,
    };
    if (!p || !p->columns || !p->cell) return res;

    TableState* st = table_state(p->id);
    if (!st) return res;

    s_progress_count = 0;

    const UiStyle* ts = ui_style_merge(TABLE_CLASS_DEFAULT, p->class);
    const UiStyle* hs = ui_style_merge(TABLE_HEADER_DEFAULT, p->header_class);
    const UiStyle* rs = ui_style_merge(TABLE_ROW_DEFAULT, p->row_class);

    float row_h = p->row_height > 0.0f ? p->row_height : 26.0f;
    bool show_header = p->hide_header ? false : true;

    // Geometry comes from last frame's solved rect. On the first frame it is
    // zero, so the table renders its header and no rows, then settles.
    char body_id[96];
    snprintf(body_id, sizeof(body_id), "%s.body", p->id ? p->id : "tbl");
    Rect body_rect = uil_element_rect(body_id, 0);

    char hdr_id[96];
    snprintf(hdr_id, sizeof(hdr_id), "%s.header", p->id ? p->id : "tbl");
    Rect hdr_rect = uil_element_rect(hdr_id, 0);

    float avail = body_rect.w > 0.0f ? body_rect.w : 600.0f;
    ColLayout cols[TABLE_MAX_COLUMNS];
    float total_w = 0.0f;
    size_t col_count = layout_columns(p, avail, cols, TABLE_MAX_COLUMNS, &total_w);

    handle_column_resize(p, cols, col_count, st, hdr_rect, &res);

    // Scroll. Owned here so the visible range is known before emitting rows.
    float view_h = body_rect.h > 0.0f ? body_rect.h : row_h * 10.0f;
    float content_h = (float)p->row_count * row_h;
    float max_scroll = content_h > view_h ? content_h - view_h : 0.0f;

    bool pointer_inside = body_rect.h > 0.0f && input_hover(body_rect);
    if (pointer_inside) {
        Vec2 sc = input_scroll();
        if (sc.y != 0.0f) st->scroll_y -= sc.y * row_h * 3.0f;
    }
    if (st->scroll_y > max_scroll) st->scroll_y = max_scroll;
    if (st->scroll_y < 0.0f)       st->scroll_y = 0.0f;

    // Virtualization: one row of overscan each side so a partially scrolled row
    // is present rather than popping in.
    size_t first = (size_t)(st->scroll_y / row_h);
    if (first > 0) first--;
    size_t visible = (size_t)(view_h / row_h) + 3;
    if (first > p->row_count) first = p->row_count;
    if (first + visible > p->row_count) visible = p->row_count - first;

    res.first_visible_row = first;
    res.visible_row_count = visible;

    Color divider;
    if (!ui_style_get_token("border-subtle", &divider)) divider = (Color){0};
    Color accent;
    if (!ui_style_get_token("accent", &accent)) accent = (Color){0};
    Color primary;
    if (!ui_style_get_token("primary", &primary)) primary = accent;
    Color stripe = color_with_alpha(ui_style_fg(rs, UI_STATE_BASE), 0.03f);

    UiBoxDesc outer = ui_style_box(ts, NULL, 0, UI_STATE_BASE);
    outer.w = UI_GROW;
    outer.h = UI_GROW;

    uil__open(1 /* col */, outer);

    if (show_header) {
        UI_BOX(.id = hdr_id, .w = UI_GROW, .h = UI_PX(row_h + 4.0f), .clip_x = true) {
            emit_header(p, hs, cols, col_count, row_h + 4.0f, st, &res);
        }
        UI_BOX(.w = UI_GROW, .h = UI_PX(1.0f), .bg = divider) {}
    }

    // Body. Rows are absolutely offset inside it by the fractional scroll
    // remainder, so scrolling is smooth rather than snapping row to row.
    UI_BOX(.id = body_id, .w = UI_GROW, .h = UI_GROW, .clip_x = true, .clip_y = true) {
        float frac = st->scroll_y - (float)first * row_h;

        UI_COL(.floating = true, .float_attach = UI_ATTACH_TOP_LEFT,
               // Without float_clip_to_parent the rows escape the viewport and
               // draw over whatever follows the table.
               .float_clip_to_parent = true,
               .float_offset = { -st->scroll_x, -frac },
               .w = UI_PX(total_w > avail ? total_w : avail),
               .h = UI_PX((float)visible * row_h)) {

            for (size_t vi = 0; vi < visible; vi++) {
                size_t row = first + vi;

                char row_id[96];
                snprintf(row_id, sizeof(row_id), "%s.row", p->id ? p->id : "tbl");
                bool hovered = uil_element_hovered(row_id, (int32_t)row);
                bool selected = (p->selected_row && *p->selected_row == (int)row);

                UiStateKind state = selected ? UI_STATE_SELECTED
                                  : hovered  ? UI_STATE_HOVER : UI_STATE_BASE;

                Color bg = selected ? primary
                         : hovered  ? accent
                         : (p->striped && (row & 1)) ? stripe : (Color){0};

                UiBoxDesc rowbox = ui_style_box(rs, row_id, (int32_t)row, state);
                rowbox.w = UI_GROW;
                rowbox.h = UI_PX(row_h);
                rowbox.bg = bg;
                rowbox.pad_l = 0.0f;
                rowbox.pad_r = 0.0f;

                uil__open(0, rowbox);
                for (size_t ci = 0; ci < col_count; ci++) {
                    UiCell cell = {0};
                    p->cell(row, (size_t)cols[ci].index, &cell, p->user);
                    emit_cell(&cell, &p->columns[cols[ci].index], cols[ci].width,
                              row_h, rs, row, (size_t)cols[ci].index);
                }
                uil__close();

                if (p->row_dividers) {
                    UI_BOX(.w = UI_GROW, .h = UI_PX(1.0f), .bg = divider) {}
                }

                // Interaction. Resizing suppresses row clicks so a drag that
                // starts on the header edge does not also select a row.
                if (hovered && st->resizing_col < 0) {
                    if (input_mouse_pressed(INPUT_MOUSE_LEFT)) {
                        res.clicked_row = (int)row;
                        if (p->selected_row) *p->selected_row = (int)row;
                        if (input_click_count() >= 2 && st->last_click_row == (int)row) {
                            res.activated_row = (int)row;
                        }
                        st->last_click_row = (int)row;
                    }
                    if (input_mouse_pressed(INPUT_MOUSE_RIGHT)) {
                        res.context_row = (int)row;
                        if (p->selected_row) *p->selected_row = (int)row;
                    }
                }
            }
        }
    }

    uil__close();
    return res;
}

void ui_table_autosize_columns(UiTableColumn* cols, size_t count,
                              UiTableCellFn cell, void* user, size_t sample_rows) {
    if (!cols) return;

    for (size_t c = 0; c < count; c++) {
        if (!cols[c].visible) continue;

        // Title sets the floor; a column never gets narrower than its own header.
        float w = 24.0f;
        if (cols[c].title) w += (float)strlen(cols[c].title) * 7.0f;

        if (cell) {
            for (size_t r = 0; r < sample_rows; r++) {
                UiCell cv = {0};
                cell(r, c, &cv, user);
                if (cv.text) {
                    float cw = 24.0f + (float)strlen(cv.text) * 7.0f;
                    if (cw > w) w = cw;
                }
            }
        }
        float min_w = cols[c].min_width > 0.0f ? cols[c].min_width : 40.0f;
        cols[c].width = w > min_w ? w : min_w;
        cols[c].grow = false;
    }
}

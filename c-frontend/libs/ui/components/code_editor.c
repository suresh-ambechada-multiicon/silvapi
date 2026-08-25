#include "ui/components/code_editor.h"
#include "input/input.h"
#include "gfx/gfx.h"

#define CE_CLASS_DEFAULT "row w-full h-full bg-surface"
#define CE_MAX_TOKENS    256
#define CE_GUTTER_PAD    10.0f
#define CE_LINE_SCRATCH  4096   // per-line copy out of the gap buffer

// ---------------------------------------------------------------------------
// Retained state. The buffer owns the text, the line index, and undo history,
// so this holds only what is genuinely view state.
// ---------------------------------------------------------------------------

typedef struct {
    uint64_t key;
    bool     used;

    size_t caret;        // byte offset
    size_t sel_anchor;   // == caret when there is no selection
    size_t goal_col;     // column to aim for during vertical movement
    float  scroll_y;
    float  scroll_x;
    double blink_base;

    // Soft-wrap mapping: vstart[i] is the number of visual rows before logical
    // line i, so vstart[line_count] is the document's total row count. Rebuilt
    // only when the wrap width, line count, or document length changes -- a
    // resize is the expensive case and it happens once, not per frame.
    uint32_t* vstart;
    size_t    vstart_cap;
    size_t    vrow_total;
    size_t    cached_cols;
    size_t    cached_lines;
    size_t    cached_len;
} CeState;

#define CE_STATE_SLOTS 8
static CeState s_states[CE_STATE_SLOTS];

static uint64_t ce_key(const char* id) {
    uint64_t h = 1469598103934665603ULL;
    for (const char* p = id ? id : "code"; *p; p++) {
        h ^= (uint8_t)*p;
        h *= 1099511628211ULL;
    }
    return h ? h : 1;
}

static CeState* ce_state(const char* id) {
    uint64_t key = ce_key(id);
    for (size_t i = 0; i < CE_STATE_SLOTS; i++) {
        if (s_states[i].used && s_states[i].key == key) return &s_states[i];
    }
    for (size_t i = 0; i < CE_STATE_SLOTS; i++) {
        if (!s_states[i].used) {
            CeState* st = &s_states[i];
            memset(st, 0, sizeof(*st));
            st->key = key;
            st->used = true;
            return st;
        }
    }
    return NULL;
}

// Rows a line of `len` characters occupies. An empty line still occupies one.
static size_t rows_for(size_t len, size_t cols) {
    if (cols == 0) return 1;
    return len ? (len + cols - 1) / cols : 1;
}

static void rebuild_wrap(CeState* st, FredBuffer* buf, size_t line_count,
                        size_t doc_len, size_t cols) {
    if (st->vstart && st->cached_cols == cols &&
        st->cached_lines == line_count && st->cached_len == doc_len) {
        return;
    }

    if (line_count + 1 > st->vstart_cap) {
        size_t cap = line_count + line_count / 2 + 64;
        uint32_t* grown = realloc(st->vstart, cap * sizeof(uint32_t));
        if (!grown) return;
        st->vstart = grown;
        st->vstart_cap = cap;
    }

    // Line lengths come from the buffer's own index, so this is integer maths
    // over line_count entries with no text measurement at all.
    size_t total = 0;
    for (size_t i = 0; i < line_count; i++) {
        st->vstart[i] = (uint32_t)total;
        size_t ls = 0, ll = 0;
        fredbuf_get_line(buf, i, &ls, &ll);
        total += rows_for(ll, cols);
    }
    st->vstart[line_count] = (uint32_t)total;

    st->vrow_total = total;
    st->cached_cols = cols;
    st->cached_lines = line_count;
    st->cached_len = doc_len;
}

// Visual row -> (logical line, row within that line).
static void row_to_line(const CeState* st, size_t line_count, size_t vrow,
                        size_t* out_line, size_t* out_sub) {
    if (!st->vstart || line_count == 0) {
        *out_line = 0;
        *out_sub = 0;
        return;
    }
    if (vrow >= st->vrow_total) vrow = st->vrow_total - 1;

    size_t lo = 0, hi = line_count;
    while (lo + 1 < hi) {
        size_t mid = (lo + hi) / 2;
        if (st->vstart[mid] <= vrow) lo = mid;
        else hi = mid;
    }
    *out_line = lo;
    *out_sub = vrow - st->vstart[lo];
}

// A line can straddle the gap, so it is copied out rather than pointed into.
// One memcpy per visible line, which is ~30 lines of ~60 bytes a frame.
static size_t read_line(FredBuffer* buf, size_t line, char* out, size_t cap) {
    size_t start = 0, len = 0;
    fredbuf_get_line(buf, line, &start, &len);
    if (len >= cap) len = cap - 1;
    if (len > 0) fredbuf_copy_range(buf, start, len, out);
    out[len] = '\0';
    return len;
}

// ---------------------------------------------------------------------------
// Draw context. Props are COPIED: callers pass a compound literal, whose
// lifetime ends with the call, but drawing happens later during the render pass.
// ---------------------------------------------------------------------------

typedef struct {
    UiCodeEditorProps props;
    CeState* st;
    Font*    font;
    float    line_h;
    float    char_w;
    size_t   first_row;    // first visual row in view
    size_t   last_row;
    size_t   line_count;
    size_t   wrap_cols;    // 0 = no wrapping
    bool     focused;
    Color    fg, gutter_fg, caret, sel, search_bg, active_line;
    size_t   search_hits;
} CeDraw;

static CeDraw g_draw;

static void draw_gutter(Rect r, void* user) {
    (void)user;
    const CeDraw* d = &g_draw;
    if (!d->font || !d->props.buffer) return;

    size_t caret_line = 0, caret_col = 0;
    fredbuf_offset_to_line_col(d->props.buffer, d->st->caret, &caret_line, &caret_col);

    float frac = d->st->scroll_y - (float)d->first_row * d->line_h;

    r_push_clip(r);
    for (size_t vrow = d->first_row; vrow <= d->last_row && vrow < d->st->vrow_total; vrow++) {
        size_t line = 0, sub = 0;
        row_to_line(d->st, d->line_count, vrow, &line, &sub);

        float y = r.y + (float)(vrow - d->first_row) * d->line_h - frac;
        Rect cell = rect_make(r.x, y, r.w - CE_GUTTER_PAD * 0.5f, d->line_h);

        // Continuation rows get a marker instead of a repeated number, so a
        // wrapped line reads as one line rather than several.
        if (sub > 0) {
            font_draw_text_aligned(d->font, "\xC2\xBB", 0, cell, TEXT_ALIGN_RIGHT,
                                   TEXT_VALIGN_MIDDLE, d->gutter_fg);
            continue;
        }

        char num[24];
        snprintf(num, sizeof(num), "%zu", line + 1);
        font_draw_text_aligned(d->font, num, 0, cell, TEXT_ALIGN_RIGHT,
                               TEXT_VALIGN_MIDDLE,
                               line == caret_line ? d->fg : d->gutter_fg);
    }
    r_pop_clip();
}

// Draws bytes [seg_start, seg_start+seg_len) of `line`, colouring them with
// tokens computed over the whole line.
static void draw_line_slice(const CeDraw* d, const char* line, size_t len,
                            size_t seg_start, size_t seg_len, float x, float y) {
    UiCodeToken tokens[CE_MAX_TOKENS];
    size_t n = 0;
    if (d->props.tokenize) {
        n = d->props.tokenize(line, len, tokens, CE_MAX_TOKENS, d->props.tokenize_user);
    }
    if (n == 0) {
        font_draw_text(d->font, line + seg_start, seg_len, x, y, d->fg);
        return;
    }

    size_t seg_end = seg_start + seg_len;

    // Advance the pen with the width each span actually drew, instead of
    // re-measuring the line prefix for every token. Re-measuring made this
    // O(tokens * line length) per line, which was the whole cost of the frame.
    float pen = x;
    size_t cursor = seg_start;

    for (size_t i = 0; i < n; i++) {
        const UiCodeToken* t = &tokens[i];

        size_t ts = t->start;
        size_t te = t->start + t->length;
        if (te > len) te = len;

        // Clip the token to the slice; tokens wholly outside it are skipped.
        if (te <= seg_start) continue;
        if (ts >= seg_end) break;
        if (ts < seg_start) ts = seg_start;
        if (te > seg_end) te = seg_end;
        if (te <= ts) continue;

        if (ts > cursor) {
            pen += font_draw_text(d->font, line + cursor, ts - cursor, pen, y, d->fg);
            cursor = ts;
        }

        Color c = d->fg;
        if (d->props.palette && t->kind < d->props.palette_size) {
            c = d->props.palette[t->kind];
        }
        pen += font_draw_text(d->font, line + ts, te - ts, pen, y, c);
        cursor = te;
    }

    if (cursor < seg_end) {
        font_draw_text(d->font, line + cursor, seg_end - cursor, pen, y, d->fg);
    }
}

static void draw_content(Rect r, void* user) {
    (void)user;
    CeDraw* d = &g_draw;
    if (!d->font || !d->props.buffer) return;

    FredBuffer* buf = d->props.buffer;
    CeState* st = d->st;

    size_t sel_lo = st->sel_anchor < st->caret ? st->sel_anchor : st->caret;
    size_t sel_hi = st->sel_anchor < st->caret ? st->caret : st->sel_anchor;
    bool has_sel = (sel_hi > sel_lo);

    size_t caret_line = 0, caret_col = 0;
    fredbuf_offset_to_line_col(buf, st->caret, &caret_line, &caret_col);

    size_t search_len = d->props.search ? strlen(d->props.search) : 0;
    float frac = st->scroll_y - (float)d->first_row * d->line_h;

    char scratch[CE_LINE_SCRATCH];

    r_push_clip(r);

    for (size_t vrow = d->first_row; vrow <= d->last_row && vrow < st->vrow_total; vrow++) {
        size_t line = 0, sub = 0;
        row_to_line(st, d->line_count, vrow, &line, &sub);

        float y = r.y + (float)(vrow - d->first_row) * d->line_h - frac;

        size_t ls = 0, raw_len = 0;
        fredbuf_get_line(buf, line, &ls, &raw_len);
        size_t ll = read_line(buf, line, scratch, sizeof(scratch));

        // The slice of the logical line this visual row shows.
        size_t seg_start = 0;
        size_t seg_len = ll;
        if (d->wrap_cols > 0) {
            seg_start = sub * d->wrap_cols;
            if (seg_start > ll) seg_start = ll;
            seg_len = ll - seg_start;
            if (seg_len > d->wrap_cols) seg_len = d->wrap_cols;
        }
        const char* seg = scratch + seg_start;
        size_t seg_abs = ls + seg_start;    // document offset of the slice

        if (d->focused && line == caret_line && d->active_line.a > 0.0f) {
            r_draw_rect(rect_make(r.x, y, r.w, d->line_h), d->active_line);
        }

        // Selection, clipped to this slice.
        if (has_sel && sel_hi > seg_abs && sel_lo <= seg_abs + seg_len) {
            size_t a = sel_lo > seg_abs ? sel_lo - seg_abs : 0;
            size_t b = sel_hi < seg_abs + seg_len ? sel_hi - seg_abs : seg_len;
            if (b > a) {
                float ax = font_measure_text(d->font, seg, a);
                float bx = font_measure_text(d->font, seg, b);
                r_draw_rect(rect_make(r.x - st->scroll_x + ax, y, bx - ax, d->line_h),
                            d->sel);
            }
        }

        if (search_len > 0 && seg_len >= search_len) {
            for (size_t i = 0; i + search_len <= seg_len; i++) {
                if (strncasecmp(seg + i, d->props.search, search_len) != 0) continue;
                float ax = font_measure_text(d->font, seg, i);
                float bx = font_measure_text(d->font, seg, i + search_len);
                r_draw_rect(rect_make(r.x - st->scroll_x + ax, y, bx - ax, d->line_h),
                            d->search_bg);
                d->search_hits++;
                i += search_len - 1;
            }
        }

        // Tokenized against the WHOLE line, then offset into the slice: a
        // tokenizer handed only a fragment would misread a string that the wrap
        // happens to cut in half.
        draw_line_slice(d, scratch, ll, seg_start, seg_len,
                        r.x - st->scroll_x, y);
    }

    // Caret last so it sits over the text.
    if (d->focused && !d->props.readonly) {
        double t = input_time_seconds() - st->blink_base;
        if (fmod(t, 1.0) < 0.55) {
            size_t ls = 0, raw_len = 0;
            fredbuf_get_line(buf, caret_line, &ls, &raw_len);
            size_t ll = read_line(buf, caret_line, scratch, sizeof(scratch));
            size_t col = st->caret > ls ? st->caret - ls : 0;
            if (col > ll) col = ll;

            size_t sub = d->wrap_cols ? col / d->wrap_cols : 0;
            size_t in_row = d->wrap_cols ? col % d->wrap_cols : col;
            size_t vrow = (st->vstart ? st->vstart[caret_line] : caret_line) + sub;

            if (vrow >= d->first_row && vrow <= d->last_row) {
                float cx = font_measure_text(d->font, scratch + sub * (d->wrap_cols ? d->wrap_cols : 0),
                                             in_row);
                float y = r.y + (float)(vrow - d->first_row) * d->line_h - frac;
                r_draw_rect(rect_make(floorf(r.x - st->scroll_x + cx), y, 1.0f, d->line_h),
                            d->caret);
            }
        }
    }

    r_pop_clip();
}

// ---------------------------------------------------------------------------

static void drop_selection(CeState* st, FredBuffer* buf) {
    size_t lo = st->sel_anchor < st->caret ? st->sel_anchor : st->caret;
    size_t hi = st->sel_anchor < st->caret ? st->caret : st->sel_anchor;
    if (hi <= lo) return;
    fredbuf_delete_range(buf, lo, hi - lo);
    st->caret = lo;
    st->sel_anchor = lo;
}

UiCodeEditorResult ui_code_editor(const UiCodeEditorProps* p) {
    UiCodeEditorResult res = {0};
    if (!p || !p->buffer) return res;

    FredBuffer* buf = p->buffer;
    CeState* st = ce_state(p->id);
    if (!st) return res;

    const UiStyle* cs = ui_style_merge(CE_CLASS_DEFAULT, p->class);
    Font* font = uil_font(UI_FONT_MONO);
    if (!font) return res;

    float line_h = p->line_height > 0.0f ? p->line_height : font_get_line_height(font);
    float char_w = font_measure_text(font, "0", 1);
    int tab_w = p->tab_width > 0 ? p->tab_width : 4;

    // The buffer maintains its own line index and only rebuilds it when dirty.
    size_t line_count = fredbuf_line_count(buf);
    size_t doc_len = fredbuf_length(buf);
    if (st->caret > doc_len) st->caret = doc_len;
    if (st->sel_anchor > doc_len) st->sel_anchor = doc_len;

    Color fg, muted, primary, warning;
    if (!ui_style_get_token("fg", &fg))           fg = (Color){1, 1, 1, 1};
    if (!ui_style_get_token("muted", &muted))     muted = fg;
    if (!ui_style_get_token("primary", &primary)) primary = fg;
    if (!ui_style_get_token("warning", &warning)) warning = primary;

    float gutter_w = 0.0f;
    if (p->show_line_numbers) {
        int digits = 1;
        for (size_t n = line_count; n >= 10; n /= 10) digits++;
        gutter_w = (float)digits * char_w + CE_GUTTER_PAD * 2.0f;
    }

    char vp_id[96];
    snprintf(vp_id, sizeof(vp_id), "%s.vp", p->id ? p->id : "code");
    Rect vp = uil_element_rect(vp_id, 0);

    // Characters per row. Monospace makes this arithmetic; a proportional font
    // would need measured break positions and a per-line cache.
    size_t wrap_cols = 0;
    if (p->soft_wrap && char_w > 0.0f) {
        float content_w = vp.w > 0.0f ? vp.w - gutter_w : 0.0f;
        long cols = (long)(content_w / char_w);
        wrap_cols = cols > 1 ? (size_t)cols : 1;
    }
    rebuild_wrap(st, buf, line_count, doc_len, wrap_cols);

    uint64_t key = ce_key(p->id);
    bool focused = (uil_focused() == key);
    bool hovered = vp.h > 0.0f && input_hover(vp);

    char scratch[CE_LINE_SCRATCH];

    // --- pointer -------------------------------------------------------------
    if (hovered && input_mouse_pressed(INPUT_MOUSE_LEFT)) {
        uil_set_focus(key);
        focused = true;

        float rel_y = input_mouse().y - vp.y + st->scroll_y;
        size_t vrow = (size_t)(rel_y / line_h);
        size_t line = 0, sub = 0;
        row_to_line(st, line_count, vrow, &line, &sub);

        size_t ls = 0, rl = 0;
        fredbuf_get_line(buf, line, &ls, &rl);
        size_t ll = read_line(buf, line, scratch, sizeof(scratch));

        size_t seg_start = wrap_cols ? sub * wrap_cols : 0;
        if (seg_start > ll) seg_start = ll;
        size_t seg_len = ll - seg_start;
        if (wrap_cols && seg_len > wrap_cols) seg_len = wrap_cols;

        float rel_x = input_mouse().x - (vp.x + gutter_w) + st->scroll_x;
        size_t col = 0;
        float acc = 0.0f;
        while (col < seg_len) {
            float w = font_measure_text(font, scratch + seg_start + col, 1);
            if (acc + w * 0.5f > rel_x) break;
            acc += w;
            col++;
        }
        st->caret = ls + seg_start + col;
        if (!(input_mods() & INPUT_MOD_SHIFT)) st->sel_anchor = st->caret;
        st->goal_col = col;
        st->blink_base = input_time_seconds();
    }

    if (hovered) {
        Vec2 sc = input_scroll();
        if (sc.y != 0.0f) st->scroll_y -= sc.y * line_h * 3.0f;
        if (sc.x != 0.0f) st->scroll_x -= sc.x * char_w * 4.0f;
    }

    // --- keyboard ------------------------------------------------------------
    if (focused) {
        bool shift = (input_mods() & INPUT_MOD_SHIFT) != 0;
        bool ctrl  = (input_mods() & INPUT_MOD_CTRL) != 0;
        bool moved = false;

        size_t cl = 0, cc = 0;
        fredbuf_offset_to_line_col(buf, st->caret, &cl, &cc);

        if (input_key_pressed(INPUT_KEY_LEFT) && st->caret > 0)  { st->caret--; moved = true; }
        if (input_key_pressed(INPUT_KEY_RIGHT) && st->caret < doc_len) { st->caret++; moved = true; }
        if (input_key_pressed(INPUT_KEY_HOME)) {
            st->caret = fredbuf_line_col_to_offset(buf, cl, 0);
            moved = true;
        }
        if (input_key_pressed(INPUT_KEY_END)) {
            size_t ls = 0, ll = 0;
            fredbuf_get_line(buf, cl, &ls, &ll);
            st->caret = ls + ll;
            moved = true;
        }

        // Vertical movement aims at a remembered goal column, so walking down
        // past a short line and back up returns to the original column.
        long vstep = 0;
        if (input_key_pressed(INPUT_KEY_UP))   vstep = -1;
        if (input_key_pressed(INPUT_KEY_DOWN)) vstep = 1;
        if (input_key_pressed(INPUT_KEY_PAGE_UP))   vstep = -(long)(vp.h / line_h);
        if (input_key_pressed(INPUT_KEY_PAGE_DOWN)) vstep =  (long)(vp.h / line_h);

        if (vstep != 0 && line_count > 0) {
            if (!moved) st->goal_col = st->goal_col > cc ? st->goal_col : cc;
            long target = (long)cl + vstep;
            if (target < 0) target = 0;
            if (target >= (long)line_count) target = (long)line_count - 1;

            size_t ls = 0, ll = 0;
            fredbuf_get_line(buf, (size_t)target, &ls, &ll);
            size_t want = st->goal_col > cc ? st->goal_col : cc;
            st->caret = ls + (want < ll ? want : ll);
            moved = true;
        } else if (moved) {
            fredbuf_offset_to_line_col(buf, st->caret, &cl, &cc);
            st->goal_col = cc;
        }

        if (moved && !shift) st->sel_anchor = st->caret;
        if (moved) st->blink_base = input_time_seconds();

        if (ctrl && input_key_pressed(INPUT_KEY_A)) {
            st->sel_anchor = 0;
            st->caret = doc_len;
        }

        if (!p->readonly) {
            // Undo/redo come from the buffer, so they cover every edit path
            // rather than only the ones this component knows about.
            if (ctrl && input_key_pressed(INPUT_KEY_Z)) {
                size_t cur = st->caret;
                fredbuf_undo(buf, &cur);
                st->caret = st->sel_anchor = cur;
                res.changed = true;
            }
            if (ctrl && input_key_pressed(INPUT_KEY_Y)) {
                size_t cur = st->caret;
                fredbuf_redo(buf, &cur);
                st->caret = st->sel_anchor = cur;
                res.changed = true;
            }

            if (!ctrl) {
                const char* typed = input_text();
                size_t tn = strlen(typed);
                if (tn > 0) {
                    drop_selection(st, buf);
                    fredbuf_insert_string(buf, st->caret, typed, tn);
                    st->caret += tn;
                    st->sel_anchor = st->caret;
                    res.changed = true;
                }
                if (input_key_pressed(INPUT_KEY_ENTER)) {
                    drop_selection(st, buf);
                    fredbuf_insert_char(buf, st->caret, '\n');
                    st->caret++;
                    st->sel_anchor = st->caret;
                    res.changed = true;
                }
                if (input_key_pressed(INPUT_KEY_TAB)) {
                    static const char spaces[] = "        ";
                    int n = tab_w > 8 ? 8 : tab_w;
                    drop_selection(st, buf);
                    fredbuf_insert_string(buf, st->caret, spaces, (size_t)n);
                    st->caret += (size_t)n;
                    st->sel_anchor = st->caret;
                    res.changed = true;
                }
                if (input_key_pressed(INPUT_KEY_BACKSPACE)) {
                    if (st->sel_anchor != st->caret) {
                        drop_selection(st, buf);
                    } else if (st->caret > 0) {
                        fredbuf_delete_range(buf, st->caret - 1, 1);
                        st->caret--;
                        st->sel_anchor = st->caret;
                    }
                    res.changed = true;
                }
                if (input_key_pressed(INPUT_KEY_DELETE)) {
                    if (st->sel_anchor != st->caret) {
                        drop_selection(st, buf);
                    } else if (st->caret < doc_len) {
                        fredbuf_delete_range(buf, st->caret, 1);
                    }
                    res.changed = true;
                }
            }
        }

        if (res.changed) {
            line_count = fredbuf_line_count(buf);
            doc_len = fredbuf_length(buf);
            if (st->caret > doc_len) st->caret = doc_len;
            if (st->sel_anchor > doc_len) st->sel_anchor = doc_len;
            st->blink_base = input_time_seconds();
        }
    }

    // --- scroll --------------------------------------------------------------
    // Edits change line lengths, so the wrap map has to be rebuilt before the
    // scroll extent is computed from it.
    rebuild_wrap(st, buf, line_count, doc_len, wrap_cols);

    float view_h = vp.h > 0.0f ? vp.h : line_h * 20.0f;
    float content_h = (float)st->vrow_total * line_h;
    float max_scroll = content_h > view_h ? content_h - view_h : 0.0f;

    if (focused) {
        // Follow the caret in VISUAL row space, or a caret inside a wrapped line
        // scrolls to the wrong place.
        size_t cl = 0, cc = 0;
        fredbuf_offset_to_line_col(buf, st->caret, &cl, &cc);
        size_t sub = wrap_cols ? cc / wrap_cols : 0;
        size_t vrow = (st->vstart ? st->vstart[cl] : cl) + sub;

        float top = (float)vrow * line_h;
        if (top < st->scroll_y) st->scroll_y = top;
        if (top + line_h > st->scroll_y + view_h) st->scroll_y = top + line_h - view_h;
    }
    if (st->scroll_y > max_scroll) st->scroll_y = max_scroll;
    if (st->scroll_y < 0.0f) st->scroll_y = 0.0f;
    // Horizontal scroll is meaningless once lines wrap.
    if (wrap_cols) st->scroll_x = 0.0f;
    if (st->scroll_x < 0.0f) st->scroll_x = 0.0f;

    size_t first = (size_t)(st->scroll_y / line_h);
    size_t last = first + (size_t)(view_h / line_h) + 2;
    if (st->vrow_total == 0) { first = 0; last = 0; }
    else if (last >= st->vrow_total) last = st->vrow_total - 1;

    // --- draw ---------------------------------------------------------------
    g_draw = (CeDraw){
        .props = *p, .st = st, .font = font,
        .line_h = line_h, .char_w = char_w,
        .first_row = first, .last_row = last, .line_count = line_count,
        .wrap_cols = wrap_cols,
        .focused = focused,
        .fg = fg,
        .gutter_fg = color_with_alpha(muted, 0.55f),
        .caret = primary,
        .sel = color_with_alpha(primary, 0.30f),
        .search_bg = color_with_alpha(warning, 0.35f),
        .active_line = color_with_alpha(fg, 0.04f),
    };

    UiBoxDesc outer = ui_style_box(cs, vp_id, 0, UI_STATE_BASE);
    outer.w = UI_GROW;
    outer.h = p->max_height > 0.0f ? UI_PX(p->max_height) : UI_GROW;
    outer.clip_x = true;
    outer.clip_y = true;

    uil__open(0 /* row */, outer);
    if (p->show_line_numbers) uil_custom(UI_PX(gutter_w), UI_GROW, draw_gutter, NULL);
    uil_custom(UI_GROW, UI_GROW, draw_content, NULL);
    uil__close();

    size_t cl = 0, cc = 0;
    fredbuf_offset_to_line_col(buf, st->caret, &cl, &cc);
    res.line_count = line_count;
    res.caret_line = cl;
    res.caret_col = cc;
    res.caret_offset = st->caret;
    {
        size_t top_line = 0, top_sub = 0;
        row_to_line(st, line_count, first, &top_line, &top_sub);
        res.first_visible_line = top_line;
    }
    res.visible_rows = (st->vrow_total && last >= first) ? (last - first + 1) : 0;
    res.total_rows = st->vrow_total;
    res.search_hits = g_draw.search_hits;
    return res;
}

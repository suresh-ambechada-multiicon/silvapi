#include "text_internal.h"
#include "base/arena.h"

// ---------------------------------------------------------------------------
// Line breaking, wrapped drawing, and caret mapping.
//
// Everything here measures via text_shape(), the same path drawing uses, so a
// line's laid-out width is exactly its drawn width. The previous implementation
// accumulated font_measure_char() per byte -- no kerning, no shaping, no UTF-8 --
// so wrapped text overflowed its box and the caret drifted from its glyph.
// ---------------------------------------------------------------------------

static void push_line(TextLayout* l, size_t start, size_t len, float width, Arena* arena) {
    if (l->line_count >= l->line_cap) {
        size_t cap = l->line_cap ? l->line_cap * 2 : 8;
        if (arena) {
            TextLine* fresh = (TextLine*)arena_push(arena, cap * sizeof(TextLine));
            if (!fresh) return;
            if (l->lines && l->line_count) memcpy(fresh, l->lines, l->line_count * sizeof(TextLine));
            l->lines = fresh;
            l->arena_owned = true;
        } else {
            TextLine* grown = (TextLine*)realloc(l->lines, cap * sizeof(TextLine));
            if (!grown) return;
            l->lines = grown;
        }
        l->line_cap = cap;
    }

    TextLine* line = &l->lines[l->line_count];
    line->start_byte  = start;
    line->length_byte = len;
    line->width       = width;
    line->y_offset    = (float)l->line_count * l->line_height;

    if (width > l->total_width) l->total_width = width;
    l->line_count++;
}

// UAX#14-lite: is there a break opportunity *after* this codepoint?
static bool breaks_after(uint32_t cp) {
    switch (cp) {
        case ' ': case '\t':
        case '-': case '/': case '\\':
        case ',': case ';': case ':':
        case '?': case '!': case '&':
        case ')': case ']': case '}':
        case 0x2014: /* em dash */
        case 0x2013: /* en dash */
            return true;
        default:
            // CJK ideographs and Hiragana/Katakana break between characters.
            if (cp >= 0x3040 && cp <= 0x30FF) return true;   // kana
            if (cp >= 0x4E00 && cp <= 0x9FFF) return true;   // CJK unified
            if (cp >= 0xAC00 && cp <= 0xD7AF) return true;   // Hangul
            return false;
    }
}

// Advance width of the shaped glyphs covering bytes [from, to).
static float measure_span(Font* font, const char* text, size_t from, size_t to) {
    if (to <= from) return 0.0f;
    return text_shape(font, text + from, to - from).width;
}

TextLayout font_layout_text_wrapped(Font* font, const char* text, size_t len,
                                    float max_width, float line_height, Arena* arena) {
    TextLayout layout = {0};
    if (!font) return layout;

    layout.line_height = line_height > 0.0f ? line_height : font_get_line_height(font);

    if (!text || len == 0) {
        push_line(&layout, 0, 0, 0.0f, arena);
        layout.total_height = layout.line_height;
        return layout;
    }

    size_t line_start = 0;
    size_t last_break = 0;     // byte offset just after the last break opportunity
    bool   have_break = false;

    size_t i = 0;
    while (i < len) {
        size_t cp_start = i;
        uint32_t cp = utf8_next(text, len, &i);

        if (cp == '\r') continue;

        if (cp == '\n') {
            push_line(&layout, line_start, cp_start - line_start,
                      measure_span(font, text, line_start, cp_start), arena);
            line_start = i;
            have_break = false;
            continue;
        }

        // Width of the line including the codepoint just consumed. Measured on
        // the whole span so kerning and ligatures across the span are included.
        float w = measure_span(font, text, line_start, i);

        if (max_width > 0.0f && w > max_width && i > line_start) {
            if (have_break && last_break > line_start) {
                // Break at the last opportunity; trailing spaces do not count
                // toward the visible line width.
                size_t end = last_break;
                size_t vis = end;
                while (vis > line_start && (text[vis - 1] == ' ' || text[vis - 1] == '\t')) vis--;

                push_line(&layout, line_start, vis - line_start,
                          measure_span(font, text, line_start, vis), arena);
                line_start = end;
                have_break = false;
                // Re-examine the current codepoint on the new line.
                i = cp_start;
                continue;
            }

            // A single unbreakable run wider than the box: break before this
            // codepoint, which is always a UTF-8 boundary.
            if (cp_start > line_start) {
                push_line(&layout, line_start, cp_start - line_start,
                          measure_span(font, text, line_start, cp_start), arena);
                line_start = cp_start;
                have_break = false;
                continue;
            }
            // Even one codepoint does not fit; emit it alone to make progress.
            push_line(&layout, line_start, i - line_start, w, arena);
            line_start = i;
            have_break = false;
            continue;
        }

        if (breaks_after(cp)) {
            last_break = i;
            have_break = true;
        }
    }

    push_line(&layout, line_start, len - line_start,
              measure_span(font, text, line_start, len), arena);

    layout.total_height = (float)layout.line_count * layout.line_height;
    return layout;
}

void font_layout_free(TextLayout* layout) {
    if (!layout) return;
    // Arena-allocated line arrays are owned by the arena; freeing them here
    // would be a double free when the arena is reset.
    if (layout->lines && !layout->arena_owned) free(layout->lines);
    layout->lines = NULL;
    layout->line_count = 0;
    layout->line_cap = 0;
}

void font_draw_text_wrapped(Font* font, const TextLayout* layout, const char* text,
                            float x, float y, Color color) {
    if (!font || !layout || !text) return;
    for (size_t i = 0; i < layout->line_count; i++) {
        const TextLine* line = &layout->lines[i];
        if (line->length_byte == 0) continue;
        font_draw_text(font, text + line->start_byte, line->length_byte,
                       x, y + line->y_offset, color);
    }
}

void font_draw_text_wrapped_aligned(Font* font, const TextLayout* layout, const char* text,
                                    Rect box, TextAlign align, Color color) {
    if (!font || !layout || !text) return;
    for (size_t i = 0; i < layout->line_count; i++) {
        const TextLine* line = &layout->lines[i];
        if (line->length_byte == 0) continue;

        float x = box.x;
        if (align == TEXT_ALIGN_CENTER)     x = box.x + (box.w - line->width) * 0.5f;
        else if (align == TEXT_ALIGN_RIGHT) x = box.x + box.w - line->width;

        font_draw_text(font, text + line->start_byte, line->length_byte,
                       x, box.y + line->y_offset, color);
    }
}

void font_layout_get_cursor_pos(const TextLayout* layout, Font* font, const char* text,
                                size_t byte_idx, float* out_x, float* out_y, float* out_line_h) {
    if (out_x) *out_x = 0.0f;
    if (out_y) *out_y = 0.0f;
    if (out_line_h) *out_line_h = font ? font_get_line_height(font) : 16.0f;
    if (!layout || !font || !text || layout->line_count == 0) return;

    if (out_line_h) *out_line_h = layout->line_height;

    for (size_t i = 0; i < layout->line_count; i++) {
        const TextLine* line = &layout->lines[i];
        bool last = (i + 1 == layout->line_count);

        if (last || byte_idx < layout->lines[i + 1].start_byte) {
            size_t from = line->start_byte;
            size_t to = byte_idx;
            if (to < from) to = from;
            if (to > from + line->length_byte) to = from + line->length_byte;

            if (out_x) *out_x = measure_span(font, text, from, to);
            if (out_y) *out_y = line->y_offset;
            return;
        }
    }
}

size_t font_layout_get_byte_offset_at_point(const TextLayout* layout, Font* font, const char* text,
                                            float click_x, float click_y,
                                            float text_x, float text_y) {
    if (!layout || !font || !text || layout->line_count == 0) return 0;

    float rel_y = click_y - text_y;
    float rel_x = click_x - text_x;

    size_t li = 0;
    if (rel_y > 0.0f && layout->line_height > 0.0f) {
        li = (size_t)(rel_y / layout->line_height);
        if (li >= layout->line_count) li = layout->line_count - 1;
    }

    const TextLine* line = &layout->lines[li];
    if (line->length_byte == 0) return line->start_byte;
    if (rel_x <= 0.0f) return line->start_byte;

    // Walk shaped glyphs and pick the nearest cluster boundary. Using clusters
    // keeps the result on a UTF-8 boundary and correct for ligatures.
    ShapedRun run = text_shape(font, text + line->start_byte, line->length_byte);
    float x = 0.0f;
    for (size_t i = 0; i < run.count; i++) {
        float adv = run.glyphs[i].x_advance;
        if (rel_x < x + adv * 0.5f) {
            return line->start_byte + run.glyphs[i].cluster;
        }
        x += adv;
    }
    return line->start_byte + line->length_byte;
}

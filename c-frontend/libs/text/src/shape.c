#include "text_internal.h"

// ---------------------------------------------------------------------------
// Shaped-run cache.
//
// The previous engine called hb_buffer_create() + hb_shape() inside both
// font_measure_text() and font_draw_text(), so every label in the UI was shaped
// at least twice per frame and allocated a HarfBuzz buffer each time. A UI with
// a few hundred text items paid thousands of shaping passes per frame.
//
// Here a string is shaped once and the glyph array is kept. Entries carry the
// frame they were last touched; an entry is only evicted when its slot is
// wanted by a different string AND it has not been used this frame or last, so
// a pointer handed out during a frame stays valid for that frame.
// ---------------------------------------------------------------------------

typedef struct {
    uint64_t hash;        // 0 = empty
    uint32_t font_id;
    uint32_t byte_len;
    uint32_t glyph_start; // index into s_glyph_pool
    uint32_t glyph_count;
    float    width;
    uint32_t last_frame;
} ShapeEntry;

static ShapeEntry  s_entries[SHAPE_CACHE_SLOTS];
static ShapedGlyph s_glyph_pool[SHAPE_GLYPH_POOL];
static uint32_t    s_pool_head = 0;
static uint32_t    s_frame = 1;
static size_t      s_entry_count = 0;
static size_t      s_hits = 0;
static size_t      s_misses = 0;

// Scratch used when a string is too long to cache, so callers still get a
// stable pointer for the duration of the call.
static ShapedGlyph s_scratch[SHAPE_MAX_RUN_BYTES * 2];

static hb_buffer_t* s_hb_buf = NULL;

void text_shape_cache_init(void) {
    if (!s_hb_buf) s_hb_buf = hb_buffer_create();
    memset(s_entries, 0, sizeof(s_entries));
    s_pool_head = 0;
    s_frame = 1;
    s_entry_count = 0;
}

void text_shape_cache_shutdown(void) {
    if (s_hb_buf) {
        hb_buffer_destroy(s_hb_buf);
        s_hb_buf = NULL;
    }
    memset(s_entries, 0, sizeof(s_entries));
    s_pool_head = 0;
    s_entry_count = 0;
}

void text_shape_cache_next_frame(void) {
    s_frame++;
    // The glyph pool is a bump allocator. Wrapping it invalidates every entry,
    // so entries are dropped wholesale rather than left pointing at reused
    // pool slots.
    if (s_pool_head > SHAPE_GLYPH_POOL - (SHAPE_GLYPH_POOL / 8)) {
        memset(s_entries, 0, sizeof(s_entries));
        s_pool_head = 0;
        s_entry_count = 0;
    }
}

void text_shape_cache_forget_font(uint32_t font_id) {
    for (size_t i = 0; i < SHAPE_CACHE_SLOTS; i++) {
        if (s_entries[i].hash && s_entries[i].font_id == font_id) {
            memset(&s_entries[i], 0, sizeof(s_entries[i]));
            if (s_entry_count) s_entry_count--;
        }
    }
}

void text_shape_cache_stats(size_t* entries, size_t* hits, size_t* misses) {
    if (entries) *entries = s_entry_count;
    if (hits)    *hits    = s_hits;
    if (misses)  *misses  = s_misses;
}

static uint64_t hash_run(uint32_t font_id, const char* text, size_t len) {
    // FNV-1a
    uint64_t h = 1469598103934665603ULL ^ ((uint64_t)font_id * 1099511628211ULL);
    for (size_t i = 0; i < len; i++) {
        h ^= (uint8_t)text[i];
        h *= 1099511628211ULL;
    }
    h ^= len * 1099511628211ULL;
    return h ? h : 1; // 0 marks an empty slot
}

// Which face in the stack can render this codepoint. Falls back to face 0 so
// unrenderable codepoints still occupy a cluster (and show .notdef) rather than
// silently disappearing, which is what the old engine did.
static uint8_t pick_face(Font* font, uint32_t codepoint) {
    for (int i = 0; i < font->face_count; i++) {
        if (font->faces[i].ft && FT_Get_Char_Index(font->faces[i].ft, codepoint) != 0) {
            return (uint8_t)i;
        }
    }
    return 0;
}

// Shapes one same-face segment, appending to `out`. Returns glyphs written.
static size_t shape_segment(Font* font, uint8_t face_slot,
                            const char* text, size_t seg_start, size_t seg_len,
                            ShapedGlyph* out, size_t out_cap, float* inout_width) {
    if (seg_len == 0 || out_cap == 0) return 0;
    hb_font_t* hbf = font->faces[face_slot].hb;
    if (!hbf) return 0;

    hb_buffer_clear_contents(s_hb_buf);
    // The whole string is added as context but only the segment is shaped, so
    // HarfBuzz can still see neighbouring characters for correct joining.
    hb_buffer_add_utf8(s_hb_buf, text, (int)(seg_start + seg_len), (int)seg_start, (int)seg_len);
    hb_buffer_guess_segment_properties(s_hb_buf);
    hb_shape(hbf, s_hb_buf, NULL, 0);

    unsigned int n = 0;
    hb_glyph_info_t*     info = hb_buffer_get_glyph_infos(s_hb_buf, &n);
    hb_glyph_position_t* pos  = hb_buffer_get_glyph_positions(s_hb_buf, &n);

    // HarfBuzz reports advances for the physically-sized face; the rest of the
    // engine works in logical units.
    const float inv = (g_text_cfg.dpi_scale > 0.0f) ? 1.0f / g_text_cfg.dpi_scale : 1.0f;

    size_t written = 0;
    for (unsigned int i = 0; i < n && written < out_cap; i++) {
        ShapedGlyph* g = &out[written++];
        g->glyph     = info[i].codepoint;
        g->cluster   = info[i].cluster;
        g->x_advance = (float)pos[i].x_advance / 64.0f * inv;
        g->x_offset  = (float)pos[i].x_offset  / 64.0f * inv;
        g->y_offset  = (float)pos[i].y_offset  / 64.0f * inv;
        g->face_slot = face_slot;
        *inout_width += g->x_advance;
    }
    return written;
}

// Itemizes `text` into face-homogeneous segments and shapes each.
static size_t shape_into(Font* font, const char* text, size_t len,
                        ShapedGlyph* out, size_t out_cap, float* out_width) {
    float width = 0.0f;
    size_t written = 0;

    size_t seg_start = 0;
    uint8_t seg_face = 0;
    bool have_seg = false;

    size_t i = 0;
    while (i < len) {
        size_t cp_start = i;
        uint32_t cp = utf8_next(text, len, &i);

        // Tab is not a shapable glyph in most UI fonts; it is expanded by the
        // layout layer, so keep it on the primary face.
        uint8_t face = (cp == '\t' || cp == '\n' || cp == '\r') ? 0 : pick_face(font, cp);

        if (!have_seg) {
            seg_start = cp_start;
            seg_face = face;
            have_seg = true;
        } else if (face != seg_face) {
            written += shape_segment(font, seg_face, text, seg_start, cp_start - seg_start,
                                     out + written, out_cap - written, &width);
            seg_start = cp_start;
            seg_face = face;
        }
    }

    if (have_seg && seg_start < len) {
        written += shape_segment(font, seg_face, text, seg_start, len - seg_start,
                                 out + written, out_cap - written, &width);
    }

    if (out_width) *out_width = width;
    return written;
}

ShapedRun text_shape(Font* font, const char* text, size_t len) {
    ShapedRun run = {0};
    if (!font || !text || len == 0 || font->face_count == 0) return run;

    // Long strings bypass the cache: they are rare, they would evict many
    // useful short entries, and the glyph pool is a fixed budget.
    if (len > SHAPE_MAX_RUN_BYTES) {
        float w = 0.0f;
        size_t n = shape_into(font, text, len, s_scratch,
                              sizeof(s_scratch) / sizeof(s_scratch[0]), &w);
        run.glyphs = s_scratch;
        run.count = n;
        run.width = w;
        return run;
    }

    uint64_t h = hash_run(font->id, text, len);
    size_t base = (size_t)(h % SHAPE_CACHE_SLOTS);

    ShapeEntry* victim = NULL;
    for (int probe = 0; probe < 8; probe++) {
        ShapeEntry* e = &s_entries[(base + (size_t)probe) % SHAPE_CACHE_SLOTS];

        if (e->hash == h && e->font_id == font->id && e->byte_len == (uint32_t)len) {
            e->last_frame = s_frame;
            s_hits++;
            run.glyphs = &s_glyph_pool[e->glyph_start];
            run.count  = e->glyph_count;
            run.width  = e->width;
            return run;
        }

        if (e->hash == 0) { victim = e; break; }

        // Only reuse a slot whose pointer nobody can still be holding.
        if (!victim && e->last_frame + 1 < s_frame) victim = e;
    }

    s_misses++;

    float w = 0.0f;
    // Worst case one glyph per byte; shaping can produce fewer (ligatures) or,
    // rarely, more (decomposition), so headroom is reserved.
    size_t cap = len * 2 + 8;
    if (s_pool_head + cap > SHAPE_GLYPH_POOL || !victim) {
        // No room to cache: shape into scratch and return it uncached.
        size_t n = shape_into(font, text, len, s_scratch,
                              sizeof(s_scratch) / sizeof(s_scratch[0]), &w);
        run.glyphs = s_scratch;
        run.count = n;
        run.width = w;
        return run;
    }

    ShapedGlyph* dst = &s_glyph_pool[s_pool_head];
    size_t n = shape_into(font, text, len, dst, cap, &w);

    if (victim->hash == 0) s_entry_count++;
    victim->hash        = h;
    victim->font_id     = font->id;
    victim->byte_len    = (uint32_t)len;
    victim->glyph_start = s_pool_head;
    victim->glyph_count = (uint32_t)n;
    victim->width       = w;
    victim->last_frame  = s_frame;

    s_pool_head += (uint32_t)n;

    run.glyphs = dst;
    run.count  = n;
    run.width  = w;
    return run;
}

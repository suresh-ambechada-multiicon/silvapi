#include "text_internal.h"

static FT_Library s_ft = NULL;
static uint32_t   s_next_font_id = 1;

Font*      g_text_fonts[TEXT_MAX_FONTS];
size_t     g_text_font_count = 0;
TextConfig g_text_cfg = { .dpi_scale = 1.0f, .hinting = TEXT_HINT_NORMAL, .subpixel_buckets = 1 };

TextConfig text_config_default(void) {
    TextConfig c = {0};
    c.dpi_scale = 1.0f;
    // Full hinting plus whole-pixel positioning is what makes small UI text
    // look crisp. Light hinting with quarter-pixel positioning spreads every
    // stem over two pixels, which reads as blur at 15px.
    c.hinting = TEXT_HINT_NORMAL;
    c.subpixel_buckets = 1;
    return c;
}

TextConfig text_get_config(void) { return g_text_cfg; }

static FT_Int32 ft_load_flags(void) {
    switch (g_text_cfg.hinting) {
        case TEXT_HINT_NONE:   return FT_LOAD_NO_BITMAP | FT_LOAD_NO_HINTING;
        case TEXT_HINT_LIGHT:  return FT_LOAD_NO_BITMAP | FT_LOAD_TARGET_LIGHT;
        case TEXT_HINT_NORMAL:
        default:               return FT_LOAD_NO_BITMAP | FT_LOAD_TARGET_NORMAL;
    }
}

static FT_Render_Mode ft_render_mode(void) {
    return g_text_cfg.hinting == TEXT_HINT_LIGHT ? FT_RENDER_MODE_LIGHT
                                                 : FT_RENDER_MODE_NORMAL;
}

static int subpixel_buckets(void) {
    int b = g_text_cfg.subpixel_buckets;
    if (b < 1) b = 1;
    if (b > TEXT_SUBPIXEL_BUCKETS) b = TEXT_SUBPIXEL_BUCKETS;
    return b;
}

FT_Library text_ft_library(void) {
    if (!s_ft) {
        if (FT_Init_FreeType(&s_ft)) {
            fprintf(stderr, "[Text] FreeType init failed\n");
            return NULL;
        }
        text_shape_cache_init();
    }
    return s_ft;
}

// ---------------------------------------------------------------------------
// UTF-8
// ---------------------------------------------------------------------------

uint32_t utf8_next(const char* s, size_t len, size_t* i) {
    if (!s || !i || *i >= len) { if (i) (*i)++; return 0xFFFD; }
    const unsigned char* p = (const unsigned char*)s;
    unsigned char c = p[*i];

    if (c < 0x80) { (*i)++; return c; }

    int extra;
    uint32_t cp;
    if      ((c & 0xE0) == 0xC0) { extra = 1; cp = c & 0x1F; }
    else if ((c & 0xF0) == 0xE0) { extra = 2; cp = c & 0x0F; }
    else if ((c & 0xF8) == 0xF0) { extra = 3; cp = c & 0x07; }
    else { (*i)++; return 0xFFFD; }  // stray continuation or invalid lead

    // Continuation bytes live at i+1 .. i+extra, so they all exist only if
    // i + extra < len.
    if (*i + (size_t)extra >= len) {
        *i = len;
        return 0xFFFD;
    }

    for (int k = 1; k <= extra; k++) {
        unsigned char cc = p[*i + (size_t)k];
        if ((cc & 0xC0) != 0x80) { (*i)++; return 0xFFFD; }
        cp = (cp << 6) | (cc & 0x3F);
    }
    *i += (size_t)extra + 1;
    return cp;
}

size_t utf8_prev_boundary(const char* s, size_t i) {
    if (!s || i == 0) return 0;
    size_t j = i - 1;
    while (j > 0 && ((unsigned char)s[j] & 0xC0) == 0x80) j--;
    return j;
}

size_t utf8_next_boundary(const char* s, size_t len, size_t i) {
    if (!s || i >= len) return len;
    size_t j = i + 1;
    while (j < len && ((unsigned char)s[j] & 0xC0) == 0x80) j++;
    return j;
}

// ---------------------------------------------------------------------------
// Glyph cache
// ---------------------------------------------------------------------------

const CachedGlyph* text_glyph(Font* font, uint8_t face_slot, uint32_t glyph_index, int bucket) {
    if (!font || glyph_index == 0) return NULL;
    if (face_slot >= (uint8_t)font->face_count) return NULL;
    if (bucket < 0 || bucket >= subpixel_buckets()) bucket = 0;

    FT_Face face = font->faces[face_slot].ft;
    if (!face) return NULL;

    uint32_t key = (glyph_index << 4) | ((uint32_t)bucket << 2) | 1u;
    uint32_t h = (key * 2654435761u) ^ ((uint32_t)face_slot * 40503u);

    for (int probe = 0; probe < 32; probe++) {
        CachedGlyph* g = &font->glyphs[(h + (uint32_t)probe) % GLYPH_CACHE_SLOTS];

        if (g->valid) {
            if (g->key == key && g->face_slot == face_slot) return g;
            continue;
        }

        // Load the outline without rendering so it can be shifted by the
        // subpixel offset before rasterization.
        if (FT_Load_Glyph(face, glyph_index, ft_load_flags())) {
            return NULL;
        }
        FT_GlyphSlot slot = face->glyph;

        int buckets = subpixel_buckets();
        if (slot->format == FT_GLYPH_FORMAT_OUTLINE && bucket != 0 && buckets > 1) {
            // 26.6 fixed point: one physical pixel is 64 units.
            FT_Outline_Translate(&slot->outline, (FT_Pos)(bucket * (64 / buckets)), 0);
        }
        if (FT_Render_Glyph(slot, ft_render_mode())) return NULL;

        int gw = (int)slot->bitmap.width;
        int gh = (int)slot->bitmap.rows;

        AtlasRegion region = {0};
        if (gw > 0 && gh > 0) {
            region = atlas_alloc(gw, gh);
            if (!region.valid) {
                fprintf(stderr, "[Text] Glyph atlas full (occupancy %.2f)\n",
                        (double)atlas_occupancy());
                return NULL;
            }
            // FreeType rows may be padded; upload honours the pitch directly
            // instead of the old one-glTexSubImage2D-per-row loop.
            atlas_upload(&region, slot->bitmap.buffer, slot->bitmap.pitch);
        }

        g->key       = key;
        g->face_slot = face_slot;
        g->region    = region;
        g->bearing_x = (float)slot->bitmap_left;
        g->bearing_y = -(float)slot->bitmap_top;
        g->valid     = true;
        font->glyph_count++;
        return g;
    }
    return NULL;
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

// `px` is the PHYSICAL raster size (already multiplied by the DPI scale).
static bool face_open(FontFace* out, const char* path, float px) {
    FT_Library lib = text_ft_library();
    if (!lib || !path) return false;

    FT_Face ft = NULL;
    if (FT_New_Face(lib, path, 0, &ft)) return false;
    if (FT_Set_Pixel_Sizes(ft, 0, (FT_UInt)(px + 0.5f))) {
        FT_Done_Face(ft);
        return false;
    }
    out->ft = ft;
    out->hb = hb_ft_font_create_referenced(ft);
    out->owns_ft = true;
    return out->hb != NULL;
}

static void compute_metrics(Font* font) {
    FT_Face face = font->faces[0].ft;
    FontMetrics m = {0};
    m.em_size = font->size;

    // FreeType reports physical pixels; everything above this layer works in
    // logical units, so each value is divided back down by the DPI scale.
    const float inv = (g_text_cfg.dpi_scale > 0.0f) ? 1.0f / g_text_cfg.dpi_scale : 1.0f;

    if (face) {
        // Scaled metrics respect the pixel size actually set, including any
        // rounding FreeType applied.
        m.ascent      =  (float)face->size->metrics.ascender  / 64.0f * inv;
        m.descent     = -(float)face->size->metrics.descender / 64.0f * inv;
        m.line_height =  (float)face->size->metrics.height    / 64.0f * inv;

        if (FT_Load_Char(face, 'H', FT_LOAD_NO_BITMAP) == 0) {
            m.cap_height = (float)face->glyph->metrics.height / 64.0f * inv;
        }
        if (FT_Load_Char(face, 'x', FT_LOAD_NO_BITMAP) == 0) {
            m.x_height = (float)face->glyph->metrics.height / 64.0f * inv;
        }
        if (FT_Load_Char(face, ' ', FT_LOAD_NO_BITMAP) == 0) {
            m.space_width = (float)face->glyph->metrics.horiAdvance / 64.0f * inv;
        }
    }

    if (m.cap_height  <= 0.0f) m.cap_height  = font->size * 0.72f;
    if (m.x_height    <= 0.0f) m.x_height    = font->size * 0.52f;
    if (m.space_width <= 0.0f) m.space_width = font->size * 0.31f;
    if (m.line_height <= 0.0f) m.line_height = font->size * 1.30f;
    if (m.ascent      <= 0.0f) m.ascent      = font->size * 0.80f;

    font->metrics = m;
}

Font* font_load(const char* file_path, float pixel_height) {
    if (!file_path || pixel_height <= 0.0f) return NULL;
    if (!text_ft_library()) return NULL;

    Font* font = calloc(1, sizeof(Font));
    if (!font) return NULL;

    font->size = pixel_height;
    font->px_size = pixel_height * (g_text_cfg.dpi_scale > 0.0f ? g_text_cfg.dpi_scale : 1.0f);
    font->id = s_next_font_id++;
    snprintf(font->path, sizeof(font->path), "%s", file_path);

    if (!face_open(&font->faces[0], file_path, font->px_size)) {
        free(font);
        return NULL;
    }
    font->face_count = 1;
    compute_metrics(font);

    if (g_text_font_count < TEXT_MAX_FONTS) g_text_fonts[g_text_font_count++] = font;
    return font;
}

bool font_add_fallback(Font* font, const char* file_path) {
    if (!font || !file_path) return false;
    if (font->face_count >= FONT_MAX_FACES) return false;

    FontFace face = {0};
    if (!face_open(&face, file_path, font->px_size)) return false;
    snprintf(font->fallback_paths[font->face_count], sizeof(font->fallback_paths[0]), "%s", file_path);
    font->faces[font->face_count++] = face;
    return true;
}

void font_add_system_fallbacks(Font* font) {
    if (!font) return;
    char* paths[FONT_MAX_FACES];
    size_t n = fontdb_fallback_chain(paths, FONT_MAX_FACES - 1);
    for (size_t i = 0; i < n; i++) {
        font_add_fallback(font, paths[i]);
        free(paths[i]);
    }
}

void font_free(Font* font) {
    if (!font) return;
    for (size_t i = 0; i < g_text_font_count; i++) {
        if (g_text_fonts[i] == font) {
            g_text_fonts[i] = g_text_fonts[--g_text_font_count];
            break;
        }
    }
    text_shape_cache_forget_font(font->id);
    for (int i = 0; i < font->face_count; i++) {
        if (font->faces[i].hb) hb_font_destroy(font->faces[i].hb);
        if (font->faces[i].ft && font->faces[i].owns_ft) FT_Done_Face(font->faces[i].ft);
    }
    free(font);
}

static Font* load_via_fontconfig(const char* family, bool bold, bool mono, float px) {
    char* path = fontdb_find(family, bold, mono);
    if (!path) return NULL;
    Font* f = font_load(path, px);
    free(path);
    if (f) font_add_system_fallbacks(f);
    return f;
}

Font* font_load_default_ui(float px)   { return load_via_fontconfig(NULL, false, false, px); }

Font* font_load_ui_family(const char* family, float px) {
    return load_via_fontconfig(family && *family ? family : NULL, false, false, px);
}
Font* font_load_bold_family(const char* family, float px) {
    return load_via_fontconfig(family && *family ? family : NULL, true, false, px);
}
Font* font_load_mono_family(const char* family, float px) {
    return load_via_fontconfig(family && *family ? family : NULL, false, true, px);
}
Font* font_load_default_bold(float px) { return load_via_fontconfig(NULL, true,  false, px); }
Font* font_load_default_mono(float px) { return load_via_fontconfig(NULL, false, true,  px); }

// ---------------------------------------------------------------------------
// Metrics & measurement
// ---------------------------------------------------------------------------

FontMetrics font_metrics(const Font* font) {
    if (!font) { FontMetrics m = {0}; m.em_size = 15.0f; m.line_height = 19.0f; return m; }
    return font->metrics;
}

float font_get_height(Font* font)      { return font ? font->size : 15.0f; }
float font_get_line_height(Font* font) { return font ? font->metrics.line_height : 19.0f; }

float font_measure_text(Font* font, const char* text, size_t len) {
    if (!font || !text) return 0.0f;
    if (len == 0) len = strlen(text);
    if (len == 0) return 0.0f;
    return text_shape(font, text, len).width;
}

float font_measure_char(Font* font, char c) {
    if (!font) return 8.0f;
    if (c == '\t') return font->metrics.space_width * 4.0f;
    char buf[2] = { c, 0 };
    return font_measure_text(font, buf, 1);
}

size_t font_fit_prefix(Font* font, const char* text, size_t len, float max_width) {
    if (!font || !text) return 0;
    if (len == 0) len = strlen(text);
    if (len == 0 || max_width <= 0.0f) return 0;

    ShapedRun run = text_shape(font, text, len);
    float w = 0.0f;
    for (size_t i = 0; i < run.count; i++) {
        if (w + run.glyphs[i].x_advance > max_width) {
            // Clusters are byte offsets, so this is always a UTF-8 boundary.
            return run.glyphs[i].cluster;
        }
        w += run.glyphs[i].x_advance;
    }
    return len;
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

// Baseline for a given vertical alignment inside `box`.
static float baseline_for(Font* font, Rect box, TextVAlign valign) {
    const FontMetrics* m = &font->metrics;
    switch (valign) {
        case TEXT_VALIGN_TOP:      return box.y + m->ascent;
        case TEXT_VALIGN_BASELINE: return box.y;
        case TEXT_VALIGN_BOTTOM:   return box.y + box.h - m->descent;
        case TEXT_VALIGN_MIDDLE:
        default:
            // Centre the capital band, not the em box: optically this is what
            // reads as vertically centred for UI labels.
            return box.y + (box.h + m->cap_height) * 0.5f;
    }
}

// Glyph rasters are in physical pixels; positions and advances are logical.
// The bridge is done here: the pen is converted to physical pixels to choose a
// whole-pixel raster origin (and a subpixel variant when enabled), then the
// quad is converted back to logical space. Snapping in physical space is what
// keeps a glyph sampled 1:1 with the framebuffer instead of resampled.
static float draw_run(Font* font, ShapedRun run, float pen_x, float baseline_y, Color color) {
    const float dpi = g_text_cfg.dpi_scale > 0.0f ? g_text_cfg.dpi_scale : 1.0f;
    const float inv = 1.0f / dpi;
    const int   buckets = subpixel_buckets();

    float x = pen_x;
    float baseline_phys = floorf((baseline_y) * dpi + 0.5f);

    for (size_t i = 0; i < run.count; i++) {
        const ShapedGlyph* sg = &run.glyphs[i];

        float gx_phys = (x + sg->x_offset) * dpi;
        float gx_floor = floorf(gx_phys);
        int bucket = 0;
        if (buckets > 1) {
            bucket = (int)((gx_phys - gx_floor) * (float)buckets);
            if (bucket < 0) bucket = 0;
            if (bucket >= buckets) bucket = buckets - 1;
        } else {
            // Whole-pixel positioning: round rather than truncate so glyphs do
            // not drift consistently left.
            gx_floor = floorf(gx_phys + 0.5f);
        }

        const CachedGlyph* g = text_glyph(font, sg->face_slot, sg->glyph, bucket);
        if (g && g->region.valid && g->region.w > 0) {
            float qx_phys = gx_floor + g->bearing_x;
            float qy_phys = baseline_phys - sg->y_offset * dpi + g->bearing_y;

            Rect dst = rect_make(qx_phys * inv, qy_phys * inv,
                                 (float)g->region.w * inv, (float)g->region.h * inv);
            Rect uv  = rect_make(g->region.u0, g->region.v0,
                                 g->region.u1 - g->region.u0,
                                 g->region.v1 - g->region.v0);
            r_draw_alpha_quad(dst, uv, atlas_texture_id(), color);
        }
        x += sg->x_advance;
    }
    return x - pen_x;
}

// ---------------------------------------------------------------------------
// Config application
// ---------------------------------------------------------------------------

void text_set_config(TextConfig cfg) {
    if (cfg.dpi_scale <= 0.0f) cfg.dpi_scale = 1.0f;
    if (cfg.subpixel_buckets < 1) cfg.subpixel_buckets = 1;
    if (cfg.subpixel_buckets > TEXT_SUBPIXEL_BUCKETS) cfg.subpixel_buckets = TEXT_SUBPIXEL_BUCKETS;

    bool raster_changed = (cfg.dpi_scale != g_text_cfg.dpi_scale) ||
                          (cfg.hinting != g_text_cfg.hinting) ||
                          (cfg.subpixel_buckets != g_text_cfg.subpixel_buckets);

    g_text_cfg = cfg;
    if (!raster_changed) return;

    // Every cached raster and shaped run was produced under the old settings.
    for (size_t i = 0; i < g_text_font_count; i++) {
        Font* f = g_text_fonts[i];
        if (!f) continue;

        float px = f->size * cfg.dpi_scale;
        f->px_size = px;

        for (int k = 0; k < f->face_count; k++) {
            if (f->faces[k].ft) FT_Set_Pixel_Sizes(f->faces[k].ft, 0, (FT_UInt)(px + 0.5f));
            // HarfBuzz caches the face's scale, so it must be told too.
            if (f->faces[k].hb) hb_ft_font_changed(f->faces[k].hb);
        }

        memset(f->glyphs, 0, sizeof(f->glyphs));
        f->glyph_count = 0;
        compute_metrics(f);
        text_shape_cache_forget_font(f->id);
    }
}

float font_draw_text(Font* font, const char* text, size_t len, float x, float y, Color color) {
    if (!font || !text) return 0.0f;
    if (len == 0) len = strlen(text);
    if (len == 0) return 0.0f;

    ShapedRun run = text_shape(font, text, len);
    // Legacy contract: `y` is the top of an em box of height font_get_height(),
    // with capitals optically centred inside it.
    float baseline = y + (font->size + font->metrics.cap_height) * 0.5f;
    return draw_run(font, run, x, baseline, color);
}

float font_draw_text_aligned(Font* font, const char* text, size_t len, Rect box,
                             TextAlign align, TextVAlign valign, Color color) {
    if (!font || !text) return 0.0f;
    if (len == 0) len = strlen(text);
    if (len == 0) return 0.0f;

    ShapedRun run = text_shape(font, text, len);

    float x = box.x;
    if (align == TEXT_ALIGN_CENTER)     x = box.x + (box.w - run.width) * 0.5f;
    else if (align == TEXT_ALIGN_RIGHT) x = box.x + box.w - run.width;

    return draw_run(font, run, x, baseline_for(font, box, valign), color);
}

float font_draw_text_ellipsized(Font* font, const char* text, size_t len, Rect box,
                                TextAlign align, TextVAlign valign, Color color) {
    if (!font || !text) return 0.0f;
    if (len == 0) len = strlen(text);
    if (len == 0) return 0.0f;

    ShapedRun full = text_shape(font, text, len);
    if (full.width <= box.w) {
        return font_draw_text_aligned(font, text, len, box, align, valign, color);
    }

    static const char* ELLIPSIS = "\xE2\x80\xA6"; // U+2026
    float ell_w = font_measure_text(font, ELLIPSIS, 3);
    float avail = box.w - ell_w;
    if (avail <= 0.0f) {
        return font_draw_text_aligned(font, ELLIPSIS, 3, box, align, valign, color);
    }

    size_t keep = font_fit_prefix(font, text, len, avail);
    // Trim a trailing space so the ellipsis hugs the last visible word.
    while (keep > 0 && (text[keep - 1] == ' ' || text[keep - 1] == '\t')) keep--;

    ShapedRun head = text_shape(font, text, keep);
    float total = head.width + ell_w;

    float x = box.x;
    if (align == TEXT_ALIGN_CENTER)     x = box.x + (box.w - total) * 0.5f;
    else if (align == TEXT_ALIGN_RIGHT) x = box.x + box.w - total;

    float baseline = baseline_for(font, box, valign);
    float drawn = draw_run(font, head, x, baseline, color);
    // Re-shape the ellipsis after the head run: text_shape may have evicted it.
    drawn += draw_run(font, text_shape(font, ELLIPSIS, 3), x + drawn, baseline, color);
    return drawn;
}

// ---------------------------------------------------------------------------
// Frame hook & stats
// ---------------------------------------------------------------------------

void font_begin_frame(void) {
    text_shape_cache_next_frame();
}

TextStats font_stats(void) {
    TextStats s = {0};
    text_shape_cache_stats(&s.shape_cache_entries, &s.shape_cache_hits, &s.shape_cache_misses);
    s.atlas_occupancy = atlas_occupancy();
    return s;
}

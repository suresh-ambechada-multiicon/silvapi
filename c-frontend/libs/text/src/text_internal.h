#ifndef TEXT_INTERNAL_H
#define TEXT_INTERNAL_H

#include "text/font.h"
#include "gfx/gfx.h"
#include "gfx/atlas.h"

#include <ft2build.h>
#include FT_FREETYPE_H
#include FT_OUTLINE_H
#include <hb.h>
#include <hb-ft.h>

// Horizontal subpixel positions cached per glyph. Glyph bitmaps are rasterized
// at an integer pel origin, so drawing at a fractional pen x resamples them and
// text goes soft with uneven stem weights. Caching a quarter-pixel variant set
// keeps glyphs crisp while preserving exact fractional advances.
#define TEXT_SUBPIXEL_BUCKETS 4

#define FONT_MAX_FACES      8    // primary + fallbacks
#define GLYPH_CACHE_SLOTS   4096
#define SHAPE_CACHE_SLOTS   2048
#define SHAPE_GLYPH_POOL    (1 << 16)
#define SHAPE_MAX_RUN_BYTES 512  // longer strings bypass the cache

typedef struct {
    uint32_t    key;        // (glyph_index << 2) | subpixel_bucket, 0 = empty
    uint8_t     face_slot;
    AtlasRegion region;
    float       bearing_x;  // left side bearing, pixels
    float       bearing_y;  // top bearing relative to baseline, negative up
    bool        valid;
} CachedGlyph;

typedef struct {
    FT_Face     ft;
    hb_font_t*  hb;
    bool        owns_ft;
} FontFace;

struct Font {
    float       size;        // logical pixel size requested by the caller
    float       px_size;     // physical raster size = size * dpi_scale
    FontMetrics metrics;

    FontFace    faces[FONT_MAX_FACES];
    int         face_count;

    CachedGlyph glyphs[GLYPH_CACHE_SLOTS];
    size_t      glyph_count;

    uint32_t    id;  // distinguishes fonts in the shared shape cache

    // Reopened when the DPI scale changes.
    char        path[512];
    char        fallback_paths[FONT_MAX_FACES][512];
};

// All loaded fonts, so a config change can rebuild them.
#define TEXT_MAX_FONTS 32
extern Font*  g_text_fonts[TEXT_MAX_FONTS];
extern size_t g_text_font_count;
extern TextConfig g_text_cfg;

// One shaped glyph. `cluster` is the byte offset in the source string, used by
// wrapping and caret mapping to stay UTF-8 correct.
typedef struct {
    uint32_t glyph;
    uint32_t cluster;
    float    x_advance;
    float    x_offset;
    float    y_offset;
    uint8_t  face_slot;
} ShapedGlyph;

typedef struct {
    const ShapedGlyph* glyphs;
    size_t             count;
    float              width;
} ShapedRun;

// Shapes `text` through the font stack, itemizing by face coverage. The result
// is cached; the returned pointer is valid until the entry is evicted, which
// cannot happen before the next font_begin_frame().
ShapedRun text_shape(Font* font, const char* text, size_t len);

// Returns the glyph raster for (face, glyph, bucket), rasterizing and uploading
// to the shared atlas on first use.
const CachedGlyph* text_glyph(Font* font, uint8_t face_slot, uint32_t glyph_index, int subpixel_bucket);

void  text_shape_cache_init(void);
void  text_shape_cache_shutdown(void);
void  text_shape_cache_next_frame(void);
void  text_shape_cache_forget_font(uint32_t font_id);
void  text_shape_cache_stats(size_t* entries, size_t* hits, size_t* misses);

FT_Library text_ft_library(void);

// fontconfig-backed lookup. Returns a malloc'd path or NULL.
char* fontdb_find(const char* family, bool bold, bool mono);
// Fills `out_paths` with up to `max` fallback font paths (caller frees each).
size_t fontdb_fallback_chain(char** out_paths, size_t max);

#endif // TEXT_INTERNAL_H

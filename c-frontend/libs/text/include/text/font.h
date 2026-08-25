#ifndef TEXT_FONT_H
#define TEXT_FONT_H

#include "base/base.h"

typedef struct Font Font;
typedef struct Arena Arena;

// ---------------------------------------------------------------------------
// Text engine.
//
// A Font is a *stack*: a primary face plus fallback faces discovered through
// fontconfig. Shaping itemizes a string into runs by which face can actually
// render each codepoint, so a missing glyph falls back instead of vanishing.
//
// Every measurement goes through the same shaped-run cache that drawing uses,
// so a measured width is exactly the width that will be drawn. Wrapping and
// caret hit-testing are therefore consistent with rendering by construction —
// the previous engine measured with per-byte advances (no kerning, no shaping)
// while drawing through HarfBuzz, so wrap width and draw width disagreed.
// ---------------------------------------------------------------------------

typedef enum {
    TEXT_ALIGN_LEFT = 0,
    TEXT_ALIGN_CENTER,
    TEXT_ALIGN_RIGHT,
} TextAlign;

typedef enum {
    TEXT_VALIGN_TOP = 0,   // top of the em box at rect.y
    TEXT_VALIGN_MIDDLE,    // capital letters optically centred in the rect
    TEXT_VALIGN_BASELINE,  // baseline at rect.y
    TEXT_VALIGN_BOTTOM,
} TextVAlign;

typedef struct {
    float ascent;       // above baseline, positive
    float descent;      // below baseline, positive
    float line_height;  // recommended baseline-to-baseline distance
    float cap_height;   // height of a capital letter
    float x_height;     // height of a lowercase 'x'
    float em_size;      // pixel size the face was loaded at
    float space_width;
} FontMetrics;

// --- configuration ---------------------------------------------------------

typedef enum {
    TEXT_HINT_NONE = 0,
    TEXT_HINT_LIGHT,   // vertical-only; preserves shape, softer stems
    TEXT_HINT_NORMAL,  // snaps stems to the pixel grid; crispest at UI sizes
} TextHinting;

typedef struct {
    // Physical pixels per logical pixel. Glyphs are rasterized at
    // size * dpi_scale and drawn into logical-sized quads, so text stays sharp
    // on HiDPI instead of being a low-res raster stretched up. All metrics and
    // advances reported by this API remain in logical units.
    float dpi_scale;

    TextHinting hinting;

    // Horizontal subpixel positions cached per glyph. 1 snaps every glyph to a
    // whole physical pixel, which is crispest for small UI text. 4 spreads
    // stems across two pixels for smoother inter-glyph spacing, which suits
    // large or animated text. Clamped to 1..4.
    int subpixel_buckets;
} TextConfig;

TextConfig text_config_default(void);

// Applying a config rebuilds every loaded face at the new pixel size and clears
// the glyph and shaping caches, so it is not free: call it on startup and on
// DPI change, not per frame.
void       text_set_config(TextConfig cfg);
TextConfig text_get_config(void);

// --- lifecycle -------------------------------------------------------------

Font* font_load(const char* file_path, float pixel_height);
void  font_free(Font* font);

// Discovered through fontconfig, so these work on any distro rather than the
// previous hardcoded /usr/share/fonts paths.
Font* font_load_default_ui(float pixel_height);
Font* font_load_default_bold(float pixel_height);
Font* font_load_default_mono(float pixel_height);

// As above but with an explicit family name; NULL or "" means the system
// default for that role.
Font* font_load_ui_family(const char* family, float pixel_height);
Font* font_load_bold_family(const char* family, float pixel_height);
Font* font_load_mono_family(const char* family, float pixel_height);

// Appends a fallback face consulted when the primary cannot render a codepoint.
bool  font_add_fallback(Font* font, const char* file_path);
// Adds the system's default fallback chain. Called automatically by the
// font_load_default_* helpers.
void  font_add_system_fallbacks(Font* font);

// --- metrics ---------------------------------------------------------------

FontMetrics font_metrics(const Font* font);
float font_get_height(Font* font);       // em size (legacy name)
float font_get_line_height(Font* font);  // baseline-to-baseline

// --- measurement -----------------------------------------------------------

// `len == 0` means strlen(). Result is cached; repeated identical measurements
// in the same or later frames are hash lookups, not reshapes.
float font_measure_text(Font* font, const char* text, size_t len);

// Widest prefix of `text` that fits `max_width`, in bytes. Always lands on a
// UTF-8 boundary.
size_t font_fit_prefix(Font* font, const char* text, size_t len, float max_width);

// Kept for callers that still measure a single ASCII cell (e.g. monospace
// column maths). Prefer font_measure_text: this ignores kerning and shaping.
float font_measure_char(Font* font, char c);

// --- single-line drawing ---------------------------------------------------

// `y` is the top of an em box of height font_get_height(); capital letters end
// up optically centred in [y, y + em]. Returns the advance width drawn.
float font_draw_text(Font* font, const char* text, size_t len, float x, float y, Color color);

// Aligns within `box`. This is what widgets should use: it removes the
// per-call-site `rect.y + (rect.h - fh) * 0.5f` arithmetic that made vertical
// alignment inconsistent across the UI.
float font_draw_text_aligned(Font* font, const char* text, size_t len, Rect box,
                             TextAlign align, TextVAlign valign, Color color);

// As above, but truncates with a trailing ellipsis when the text overflows.
float font_draw_text_ellipsized(Font* font, const char* text, size_t len, Rect box,
                                TextAlign align, TextVAlign valign, Color color);

// --- multi-line layout -----------------------------------------------------

typedef struct {
    size_t start_byte;
    size_t length_byte;
    float  width;
    float  y_offset;
} TextLine;

typedef struct {
    TextLine* lines;
    size_t    line_count;
    size_t    line_cap;
    float     total_width;
    float     total_height;
    float     line_height;
    bool      arena_owned;   // true when allocated from an Arena, so no free()
} TextLayout;

// Breaks at UAX#14-style opportunities, never inside a UTF-8 sequence, and
// measures with shaped advances so a laid-out line's width matches its drawn
// width exactly. Pass `arena` to allocate the line array from an arena.
TextLayout font_layout_text_wrapped(Font* font, const char* text, size_t len,
                                    float max_width, float line_height, Arena* arena);
void font_layout_free(TextLayout* layout);

void font_draw_text_wrapped(Font* font, const TextLayout* layout, const char* text,
                            float x, float y, Color color);
void font_draw_text_wrapped_aligned(Font* font, const TextLayout* layout, const char* text,
                                    Rect box, TextAlign align, Color color);

// Caret geometry for a byte offset, and the byte offset nearest a point. Both
// are UTF-8 aware and use the same shaped advances as drawing.
void   font_layout_get_cursor_pos(const TextLayout* layout, Font* font, const char* text,
                                  size_t byte_idx, float* out_x, float* out_y, float* out_line_h);
size_t font_layout_get_byte_offset_at_point(const TextLayout* layout, Font* font, const char* text,
                                            float click_x, float click_y, float text_x, float text_y);

// --- frame hook ------------------------------------------------------------

// Advances the shaped-run cache generation so entries unused for a while can be
// evicted. Call once per frame.
void font_begin_frame(void);

// Diagnostics.
typedef struct {
    size_t shape_cache_entries;
    size_t shape_cache_hits;
    size_t shape_cache_misses;
    size_t glyphs_cached;
    float  atlas_occupancy;
} TextStats;
TextStats font_stats(void);

// --- UTF-8 helpers ---------------------------------------------------------

// Decodes one codepoint at `s[i]`, advancing `*i`. Returns 0xFFFD on bad input.
uint32_t utf8_next(const char* s, size_t len, size_t* i);
size_t   utf8_prev_boundary(const char* s, size_t i);
size_t   utf8_next_boundary(const char* s, size_t len, size_t i);

#endif // TEXT_FONT_H

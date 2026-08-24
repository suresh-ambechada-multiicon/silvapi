#include "font.h"
#include "arena.h"
#include <GL/gl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <ft2build.h>
#include FT_FREETYPE_H
#include <hb.h>
#include <hb-ft.h>

#define ATLAS_SIZE 2048
#define GLYPH_HASH_SIZE 2048

typedef struct {
    uint32_t glyph_index;
    float x0, y0, x1, y1; // Box offsets relative to cursor position
    float u0, v0, u1, v1; // Texture UVs in atlas
    float x_advance;
    bool valid;
} CachedGlyph;

struct Font {
    float size;
    float ascent;
    float descent;
    float cap_height;
    float line_gap;
    FT_Face face;
    hb_font_t* hb_font;
    GLuint texture_id;
    int atlas_cursor_x;
    int atlas_cursor_y;
    int atlas_row_h;
    unsigned char* atlas_rgba;
    CachedGlyph glyphs[GLYPH_HASH_SIZE];
};

static FT_Library s_ft_lib = NULL;

static void ensure_ft_init(void) {
    if (!s_ft_lib) {
        if (FT_Init_FreeType(&s_ft_lib)) {
            fprintf(stderr, "[Font] Failed to initialize FreeType\n");
        }
    }
}

static CachedGlyph* font_get_or_render_glyph(Font* font, uint32_t glyph_index) {
    if (!font || !font->face) return NULL;
    if (glyph_index == 0) return NULL; // Never render .notdef fallback box for whitespace or missing glyphs

    uint32_t hash = (glyph_index * 2654435761U) % GLYPH_HASH_SIZE;
    for (int probe = 0; probe < 16; probe++) {
        int idx = (hash + probe) % GLYPH_HASH_SIZE;
        if (font->glyphs[idx].valid && font->glyphs[idx].glyph_index == glyph_index) {
            return &font->glyphs[idx];
        }
        if (!font->glyphs[idx].valid) {
            // Load and render glyph with FreeType (subpixel anti-aliased with light hinting)
            FT_Error err = FT_Load_Glyph(font->face, glyph_index, FT_LOAD_RENDER | FT_LOAD_TARGET_LIGHT);
            if (err) return NULL;

            FT_GlyphSlot slot = font->face->glyph;
            int gw = slot->bitmap.width;
            int gh = slot->bitmap.rows;
            int padding = 2;

            if (font->atlas_cursor_x + gw + padding >= ATLAS_SIZE) {
                font->atlas_cursor_x = padding;
                font->atlas_cursor_y += font->atlas_row_h + padding;
                font->atlas_row_h = 0;
            }

            if (font->atlas_cursor_y + gh + padding >= ATLAS_SIZE) {
                fprintf(stderr, "[Font] Warning: Atlas texture full!\n");
                return NULL;
            }

            if (gh > font->atlas_row_h) {
                font->atlas_row_h = gh;
            }

            int ax = font->atlas_cursor_x;
            int ay = font->atlas_cursor_y;

            // Copy glyph pixels into RGBA atlas
            for (int row = 0; row < gh; row++) {
                for (int col = 0; col < gw; col++) {
                    unsigned char a = slot->bitmap.buffer[row * slot->bitmap.pitch + col];
                    int pixel_idx = ((ay + row) * ATLAS_SIZE + (ax + col)) * 4;
                    font->atlas_rgba[pixel_idx + 0] = 255;
                    font->atlas_rgba[pixel_idx + 1] = 255;
                    font->atlas_rgba[pixel_idx + 2] = 255;
                    font->atlas_rgba[pixel_idx + 3] = a;
                }
            }

            font->atlas_cursor_x += gw + padding;

            // Update GPU texture
            if (gw > 0 && gh > 0) {
                glBindTexture(GL_TEXTURE_2D, font->texture_id);
                // Sub-image upload for exact region
                for (int row = 0; row < gh; row++) {
                    glTexSubImage2D(GL_TEXTURE_2D, 0, ax, ay + row, gw, 1, GL_RGBA, GL_UNSIGNED_BYTE,
                                    &font->atlas_rgba[((ay + row) * ATLAS_SIZE + ax) * 4]);
                }
            }

            CachedGlyph* g = &font->glyphs[idx];
            g->glyph_index = glyph_index;
            g->valid = true;
            g->x0 = (float)slot->bitmap_left;
            g->y0 = -(float)slot->bitmap_top;
            g->x1 = g->x0 + (float)gw;
            g->y1 = g->y0 + (float)gh;
            g->u0 = (float)ax / (float)ATLAS_SIZE;
            g->v0 = (float)ay / (float)ATLAS_SIZE;
            g->u1 = (float)(ax + gw) / (float)ATLAS_SIZE;
            g->v1 = (float)(ay + gh) / (float)ATLAS_SIZE;
            g->x_advance = (float)slot->advance.x / 64.0f;

            return g;
        }
    }
    return NULL;
}

Font* font_load(const char* file_path, float pixel_height) {
    if (!file_path || pixel_height <= 0.0f) return NULL;
    ensure_ft_init();
    if (!s_ft_lib) return NULL;

    FT_Face face;
    if (FT_New_Face(s_ft_lib, file_path, 0, &face)) {
        return NULL;
    }

    if (FT_Set_Pixel_Sizes(face, 0, (FT_UInt)pixel_height)) {
        FT_Done_Face(face);
        return NULL;
    }

    Font* font = calloc(1, sizeof(Font));
    if (!font) {
        FT_Done_Face(face);
        return NULL;
    }

    font->size = pixel_height;
    font->face = face;
    font->hb_font = hb_ft_font_create_referenced(face);
    font->atlas_cursor_x = 2;
    font->atlas_cursor_y = 2;
    font->atlas_row_h = 0;

    // Metrics
    float scale = (float)pixel_height / (float)face->units_per_EM;
    font->ascent = (float)face->ascender * scale;
    font->descent = (float)face->descender * scale;
    font->line_gap = (float)face->height * scale - (font->ascent - font->descent);

    // Measure visual capital height (using 'H')
    if (FT_Load_Char(face, 'H', FT_LOAD_DEFAULT) == 0) {
        font->cap_height = (float)face->glyph->metrics.height / 64.0f;
    } else {
        font->cap_height = pixel_height * 0.72f;
    }

    font->atlas_rgba = calloc(1, ATLAS_SIZE * ATLAS_SIZE * 4);

    glGenTextures(1, &font->texture_id);
    glBindTexture(GL_TEXTURE_2D, font->texture_id);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, ATLAS_SIZE, ATLAS_SIZE, 0, GL_RGBA, GL_UNSIGNED_BYTE, font->atlas_rgba);

    // Pre-cache standard ASCII characters (32 to 126)
    for (uint32_t c = 32; c <= 126; c++) {
        uint32_t gi = FT_Get_Char_Index(face, c);
        if (gi != 0) {
            font_get_or_render_glyph(font, gi);
        }
    }

    return font;
}

void font_free(Font* font) {
    if (!font) return;
    if (font->texture_id) glDeleteTextures(1, &font->texture_id);
    if (font->hb_font) hb_font_destroy(font->hb_font);
    if (font->face) FT_Done_Face(font->face);
    if (font->atlas_rgba) free(font->atlas_rgba);
    free(font);
}

float font_get_height(Font* font) {
    if (!font) return 15.0f;
    return font->size;
}

float font_measure_char(Font* font, char c) {
    if (!font || !font->face) return 8.0f;
    if (c == ' ') {
        uint32_t gi = FT_Get_Char_Index(font->face, ' ');
        CachedGlyph* g = font_get_or_render_glyph(font, gi);
        return g ? g->x_advance : font->size * 0.35f;
    }
    if (c == '\t') {
        return font_measure_char(font, ' ') * 4.0f;
    }
    uint32_t gi = FT_Get_Char_Index(font->face, (unsigned char)c);
    CachedGlyph* g = font_get_or_render_glyph(font, gi);
    return g ? g->x_advance : font->size * 0.5f;
}

static char* sanitize_text_for_shaping(const char* in, size_t in_len, size_t* out_len, char* stack_buf, size_t stack_buf_sz) {
    if (!in || in_len == 0) {
        *out_len = 0;
        return NULL;
    }
    bool needs_sanitize = false;
    size_t extra = 0;
    for (size_t i = 0; i < in_len; i++) {
        if (in[i] == '\t') {
            needs_sanitize = true;
            extra += 3;
        } else if (in[i] == '\r') {
            needs_sanitize = true;
        }
    }
    if (!needs_sanitize) {
        *out_len = in_len;
        return (char*)in;
    }

    size_t needed = in_len + extra + 1;
    char* out = (needed <= stack_buf_sz) ? stack_buf : malloc(needed);
    size_t w = 0;
    for (size_t i = 0; i < in_len; i++) {
        if (in[i] == '\t') {
            out[w++] = ' ';
            out[w++] = ' ';
            out[w++] = ' ';
            out[w++] = ' ';
        } else if (in[i] == '\r') {
            continue;
        } else {
            out[w++] = in[i];
        }
    }
    out[w] = '\0';
    *out_len = w;
    return out;
}

float font_measure_text(Font* font, const char* text, size_t len) {
    if (!font || !text || !font->hb_font) return 0.0f;
    if (len == 0) len = strlen(text);
    if (len == 0) return 0.0f;

    char stack_buf[512];
    size_t shaped_len = 0;
    char* shaped_text = sanitize_text_for_shaping(text, len, &shaped_len, stack_buf, sizeof(stack_buf));
    if (!shaped_text || shaped_len == 0) return 0.0f;

    hb_buffer_t* hb_buf = hb_buffer_create();
    hb_buffer_add_utf8(hb_buf, shaped_text, (int)shaped_len, 0, (int)shaped_len);
    hb_buffer_guess_segment_properties(hb_buf);
    hb_shape(font->hb_font, hb_buf, NULL, 0);

    unsigned int glyph_count;
    hb_glyph_position_t* glyph_pos = hb_buffer_get_glyph_positions(hb_buf, &glyph_count);

    float total_w = 0.0f;
    for (unsigned int i = 0; i < glyph_count; i++) {
        total_w += (float)glyph_pos[i].x_advance / 64.0f;
    }

    hb_buffer_destroy(hb_buf);
    if (shaped_text != text && shaped_text != stack_buf) {
        free(shaped_text);
    }
    return total_w;
}

void font_draw_text(Font* font, const char* text, size_t len, float x, float y, Color color) {
    if (!font || !text || !font->hb_font) return;
    if (len == 0) len = strlen(text);
    if (len == 0) return;

    char stack_buf[512];
    size_t shaped_len = 0;
    char* shaped_text = sanitize_text_for_shaping(text, len, &shaped_len, stack_buf, sizeof(stack_buf));
    if (!shaped_text || shaped_len == 0) return;

    hb_buffer_t* hb_buf = hb_buffer_create();
    hb_buffer_add_utf8(hb_buf, shaped_text, (int)shaped_len, 0, (int)shaped_len);
    hb_buffer_guess_segment_properties(hb_buf);
    hb_shape(font->hb_font, hb_buf, NULL, 0);

    unsigned int glyph_count;
    hb_glyph_info_t* glyph_info = hb_buffer_get_glyph_infos(hb_buf, &glyph_count);
    hb_glyph_position_t* glyph_pos = hb_buffer_get_glyph_positions(hb_buf, &glyph_count);

    float cur_x = x;
    // Standard visual vertical centering:
    // When caller supplies y as the top of a line of height `font->size`,
    // baseline_y centers standard capital letters symmetrically inside [y, y + font->size].
    float baseline_y = y + (font->size + font->cap_height) * 0.5f;

    for (unsigned int i = 0; i < glyph_count; i++) {
        uint32_t glyph_index = glyph_info[i].codepoint;
        float x_offset = (float)glyph_pos[i].x_offset / 64.0f;
        float y_offset = (float)glyph_pos[i].y_offset / 64.0f;
        float x_advance = (float)glyph_pos[i].x_advance / 64.0f;

        CachedGlyph* g = font_get_or_render_glyph(font, glyph_index);
        if (g && (g->u1 > g->u0) && (g->v1 > g->v0)) {
            float gx0 = cur_x + x_offset + g->x0;
            float gy0 = baseline_y - y_offset + g->y0;
            float gx1 = cur_x + x_offset + g->x1;
            float gy1 = baseline_y - y_offset + g->y1;

            Rect dst = rect_make(gx0, gy0, gx1 - gx0, gy1 - gy0);
            Rect uv = rect_make(g->u0, g->v0, g->u1 - g->u0, g->v1 - g->v0);

            r_draw_textured_quad(dst, uv, font->texture_id, color);
        }

        cur_x += x_advance;
    }

    hb_buffer_destroy(hb_buf);
    if (shaped_text != text && shaped_text != stack_buf) {
        free(shaped_text);
    }
}

static void push_text_line(TextLayout* layout, size_t start_byte, size_t length_byte, float width, Arena* arena) {
    if (layout->line_count >= layout->line_cap) {
        size_t new_cap = layout->line_cap == 0 ? 8 : layout->line_cap * 2;
        if (arena) {
            TextLine* new_lines = (TextLine*)arena_push(arena, new_cap * sizeof(TextLine));
            if (layout->lines && layout->line_count > 0) {
                memcpy(new_lines, layout->lines, layout->line_count * sizeof(TextLine));
            }
            layout->lines = new_lines;
        } else {
            layout->lines = realloc(layout->lines, new_cap * sizeof(TextLine));
        }
        layout->line_cap = new_cap;
    }

    TextLine* line = &layout->lines[layout->line_count];
    line->start_byte = start_byte;
    line->length_byte = length_byte;
    line->width = width;
    line->y_offset = (float)layout->line_count * layout->line_height;

    if (width > layout->total_width) {
        layout->total_width = width;
    }
    layout->line_count++;
}

TextLayout font_layout_text_wrapped(Font* font, const char* text, size_t len, float max_width, float line_height, Arena* arena) {
    TextLayout layout = {0};
    if (!font) return layout;
    layout.line_height = line_height > 0.0f ? line_height : font_get_height(font) * 1.35f;

    if (!text || len == 0) {
        push_text_line(&layout, 0, 0, 0.0f, arena);
        layout.total_height = layout.line_height;
        return layout;
    }

    size_t line_start = 0;
    size_t last_break_opp = 0;
    float current_line_w = 0.0f;
    float break_opp_w = 0.0f;

    for (size_t i = 0; i < len; i++) {
        char c = text[i];

        if (c == '\r') continue;

        if (c == '\n') {
            push_text_line(&layout, line_start, i - line_start, current_line_w, arena);
            line_start = i + 1;
            last_break_opp = line_start;
            current_line_w = 0.0f;
            break_opp_w = 0.0f;
            continue;
        }

        float char_w = font_measure_char(font, c);

        // Check for line break opportunities
        if (c == ' ' || c == '\t' || c == '-' || c == '/' || c == '.' || c == ',' || c == '_' || c == ':' || c == '?' || c == '&') {
            last_break_opp = i + 1;
            break_opp_w = current_line_w + char_w;
        }

        if (max_width > 0.0f && current_line_w + char_w > max_width && (i > line_start)) {
            if (last_break_opp > line_start && last_break_opp < i + 1) {
                // Break at whitespace or delimiter boundary
                push_text_line(&layout, line_start, last_break_opp - line_start, break_opp_w, arena);
                line_start = last_break_opp;
                // Measure the remaining fragment starting on the next line
                current_line_w = font_measure_text(font, text + line_start, i - line_start + 1);
            } else {
                // Word is longer than line width: force character break
                push_text_line(&layout, line_start, i - line_start, current_line_w, arena);
                line_start = i;
                last_break_opp = i;
                current_line_w = char_w;
            }
        } else {
            current_line_w += char_w;
        }
    }

    if (line_start <= len) {
        push_text_line(&layout, line_start, len - line_start, current_line_w, arena);
    }

    layout.total_height = (float)layout.line_count * layout.line_height;
    return layout;
}

void font_layout_free(TextLayout* layout) {
    if (layout && layout->lines) {
        free(layout->lines);
        layout->lines = NULL;
        layout->line_count = 0;
        layout->line_cap = 0;
    }
}

void font_draw_text_wrapped(Font* font, const TextLayout* layout, const char* text, float x, float y, Color color) {
    if (!font || !layout || !text) return;

    for (size_t i = 0; i < layout->line_count; i++) {
        const TextLine* line = &layout->lines[i];
        if (line->length_byte == 0) continue;
        font_draw_text(font, text + line->start_byte, line->length_byte, x, y + line->y_offset, color);
    }
}

void font_layout_get_cursor_pos(const TextLayout* layout, Font* font, const char* text, size_t byte_idx, float* out_x, float* out_y, float* out_line_h) {
    if (!layout || !font || !text || layout->line_count == 0) {
        if (out_x) *out_x = 0.0f;
        if (out_y) *out_y = 0.0f;
        if (out_line_h) *out_line_h = 16.0f;
        return;
    }

    if (out_line_h) *out_line_h = layout->line_height;

    for (size_t i = 0; i < layout->line_count; i++) {
        const TextLine* line = &layout->lines[i];

        bool is_last_line = (i == layout->line_count - 1);
        // If byte_idx is before the start of the next line, it belongs strictly to this line
        if (is_last_line || byte_idx < layout->lines[i + 1].start_byte) {
            size_t sub_len = (byte_idx > line->start_byte) ? (byte_idx - line->start_byte) : 0;
            if (sub_len > line->length_byte) sub_len = line->length_byte;

            float cx = (sub_len > 0) ? font_measure_text(font, text + line->start_byte, sub_len) : 0.0f;
            if (out_x) *out_x = cx;
            if (out_y) *out_y = line->y_offset;
            return;
        }
    }

    if (out_x) *out_x = 0.0f;
    if (out_y) *out_y = 0.0f;
}

size_t font_layout_get_byte_offset_at_point(const TextLayout* layout, Font* font, const char* text, float click_x, float click_y, float text_x, float text_y) {
    if (!layout || !font || !text || layout->line_count == 0) return 0;

    float rel_y = click_y - text_y;
    float rel_x = click_x - text_x;

    size_t target_line_idx = 0;
    if (rel_y > 0.0f && layout->line_height > 0.0f) {
        target_line_idx = (size_t)(rel_y / layout->line_height);
        if (target_line_idx >= layout->line_count) {
            target_line_idx = layout->line_count - 1;
        }
    }

    const TextLine* line = &layout->lines[target_line_idx];
    if (line->length_byte == 0) return line->start_byte;

    if (rel_x <= 0.0f) return line->start_byte;

    // Binary search or probe character advances to find closest character boundary
    float cur_x = 0.0f;
    for (size_t i = 0; i < line->length_byte; i++) {
        float ch_w = font_measure_char(font, text[line->start_byte + i]);
        if (cur_x + ch_w * 0.5f >= rel_x) {
            return line->start_byte + i;
        }
        cur_x += ch_w;
    }

    return line->start_byte + line->length_byte;
}

Font* font_load_default_ui(float pixel_height) {
    const char* candidates[] = {
        "/usr/share/fonts/Adwaita/AdwaitaSans-Regular.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/google-noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf",
        NULL
    };
    for (int i = 0; candidates[i]; i++) {
        Font* f = font_load(candidates[i], pixel_height);
        if (f) return f;
    }
    return NULL;
}

Font* font_load_default_bold(float pixel_height) {
    const char* candidates[] = {
        "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/Adwaita/AdwaitaSans-Bold.ttf",
        "/usr/share/fonts/noto/NotoSans-Bold.ttf",
        "/usr/share/fonts/google-noto/NotoSans-Bold.ttf",
        "/usr/share/fonts/liberation-sans/LiberationSans-Bold.ttf",
        NULL
    };
    for (int i = 0; candidates[i]; i++) {
        Font* f = font_load(candidates[i], pixel_height);
        if (f) return f;
    }
    return font_load_default_ui(pixel_height);
}

Font* font_load_default_mono(float pixel_height) {
    const char* candidates[] = {
        "/usr/share/fonts/TTF/JetBrainsMonoNerdFontMono-Regular.ttf",
        "/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        "/usr/share/fonts/source-foundry-hack-fonts/Hack-Regular.ttf",
        "/usr/share/fonts/urw-base35/NimbusMonoPS-Regular.otf",
        NULL
    };
    for (int i = 0; candidates[i]; i++) {
        Font* f = font_load(candidates[i], pixel_height);
        if (f) return f;
    }
    return font_load_default_ui(pixel_height);
}

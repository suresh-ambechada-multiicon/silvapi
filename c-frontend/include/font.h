#ifndef FONT_H
#define FONT_H

#include "silvapi.h"
#include "gl_backend.h"

typedef struct Font Font;
typedef struct Arena Arena;

typedef struct {
    size_t start_byte;      // Starting byte offset in original string
    size_t length_byte;     // Length of line in bytes
    float width;            // Total width of this line
    float y_offset;         // Vertical offset from top of layout
} TextLine;

typedef struct {
    TextLine* lines;
    size_t line_count;
    size_t line_cap;
    float total_width;
    float total_height;
    float line_height;
} TextLayout;

Font* font_load(const char* file_path, float pixel_height);
void font_free(Font* font);

float font_get_height(Font* font);
float font_measure_text(Font* font, const char* text, size_t len);
float font_measure_char(Font* font, char c);

// Single line render
void font_draw_text(Font* font, const char* text, size_t len, float x, float y, Color color);

// Multi-line and wrapped text layout & rendering
TextLayout font_layout_text_wrapped(Font* font, const char* text, size_t len, float max_width, float line_height, Arena* arena);
void font_layout_free(TextLayout* layout);
void font_draw_text_wrapped(Font* font, const TextLayout* layout, const char* text, float x, float y, Color color);
void font_layout_get_cursor_pos(const TextLayout* layout, Font* font, const char* text, size_t byte_idx, float* out_x, float* out_y, float* out_line_h);
size_t font_layout_get_byte_offset_at_point(const TextLayout* layout, Font* font, const char* text, float click_x, float click_y, float text_x, float text_y);

// Find font on Linux system automatically
Font* font_load_default_ui(float pixel_height);
Font* font_load_default_bold(float pixel_height);
Font* font_load_default_mono(float pixel_height);

#endif // FONT_H

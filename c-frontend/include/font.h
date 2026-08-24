#ifndef FONT_H
#define FONT_H

#include "silvapi.h"
#include "gl_backend.h"

typedef struct Font Font;

Font* font_load(const char* file_path, float pixel_height);
void font_free(Font* font);

float font_get_height(Font* font);
float font_measure_text(Font* font, const char* text, size_t len);
float font_measure_char(Font* font, char c);

// Renders text starting at (x, y) baseline or top-left.
void font_draw_text(Font* font, const char* text, size_t len, float x, float y, Color color);

// Find font on Linux system automatically
Font* font_load_default_ui(float pixel_height);
Font* font_load_default_mono(float pixel_height);

#endif // FONT_H

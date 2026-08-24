#include "font.h"
#include <GL/gl.h>

#define STB_TRUETYPE_IMPLEMENTATION
#include "stb_truetype.h"

#define ATLAS_WIDTH 1024
#define ATLAS_HEIGHT 1024
#define FIRST_CHAR 32
#define NUM_CHARS 224 // 32 to 255 (ASCII + Latin-1 Extended)

struct Font {
    float size;
    float ascent;
    float descent;
    float line_gap;
    stbtt_bakedchar cdata[NUM_CHARS];
    GLuint texture_id;
};

Font* font_load(const char* file_path, float pixel_height) {
    FILE* f = fopen(file_path, "rb");
    if (!f) return NULL;

    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);

    unsigned char* ttf_buffer = malloc(size);
    if (fread(ttf_buffer, 1, size, f) != (size_t)size) {
        free(ttf_buffer);
        fclose(f);
        return NULL;
    }
    fclose(f);

    unsigned char* temp_bitmap = calloc(1, ATLAS_WIDTH * ATLAS_HEIGHT);
    Font* font = calloc(1, sizeof(Font));
    font->size = pixel_height;

    stbtt_BakeFontBitmap(ttf_buffer, 0, pixel_height, temp_bitmap, ATLAS_WIDTH, ATLAS_HEIGHT, FIRST_CHAR, NUM_CHARS, font->cdata);

    stbtt_fontinfo info;
    if (stbtt_InitFont(&info, ttf_buffer, 0)) {
        int ascent, descent, line_gap;
        stbtt_GetFontVMetrics(&info, &ascent, &descent, &line_gap);
        float scale = stbtt_ScaleForPixelHeight(&info, pixel_height);
        font->ascent = ascent * scale;
        font->descent = descent * scale;
        font->line_gap = line_gap * scale;
    } else {
        font->ascent = pixel_height * 0.8f;
        font->descent = -pixel_height * 0.2f;
        font->line_gap = pixel_height * 0.1f;
    }

    free(ttf_buffer);

    // Create RGBA texture from single-channel alpha bitmap
    unsigned char* rgba_bitmap = malloc(ATLAS_WIDTH * ATLAS_HEIGHT * 4);
    for (int i = 0; i < ATLAS_WIDTH * ATLAS_HEIGHT; i++) {
        unsigned char a = temp_bitmap[i];
        rgba_bitmap[i * 4 + 0] = 255;
        rgba_bitmap[i * 4 + 1] = 255;
        rgba_bitmap[i * 4 + 2] = 255;
        rgba_bitmap[i * 4 + 3] = a;
    }
    free(temp_bitmap);

    glGenTextures(1, &font->texture_id);
    glBindTexture(GL_TEXTURE_2D, font->texture_id);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, ATLAS_WIDTH, ATLAS_HEIGHT, 0, GL_RGBA, GL_UNSIGNED_BYTE, rgba_bitmap);
    free(rgba_bitmap);

    return font;
}

void font_free(Font* font) {
    if (!font) return;
    if (font->texture_id) {
        glDeleteTextures(1, &font->texture_id);
    }
    free(font);
}

float font_get_height(Font* font) {
    if (!font) return 14.0f;
    return font->size;
}

float font_measure_char(Font* font, char c) {
    if (!font) return 8.0f;
    if (c == ' ') {
        return font->cdata[0].xadvance;
    }
    if (c == '\t') {
        return font->cdata[0].xadvance * 4.0f;
    }
    int char_idx = (int)(unsigned char)c - FIRST_CHAR;
    if (char_idx >= 0 && char_idx < NUM_CHARS) {
        return font->cdata[char_idx].xadvance;
    }
    return font->size * 0.5f;
}

float font_measure_text(Font* font, const char* text, size_t len) {
    if (!font || !text) return 0.0f;
    if (len == 0) len = strlen(text);

    float width = 0.0f;
    for (size_t i = 0; i < len; i++) {
        char c = text[i];
        if (c == '\n' || c == '\r') break;
        width += font_measure_char(font, c);
    }
    return width;
}

void font_draw_text(Font* font, const char* text, size_t len, float x, float y, Color color) {
    if (!font || !text) return;
    if (len == 0) len = strlen(text);

    float cur_x = x;
    float baseline_y = y + font->ascent;

    for (size_t i = 0; i < len; i++) {
        char c = text[i];
        if (c == '\n' || c == '\r') break;
        if (c == ' ') {
            cur_x += font->cdata[0].xadvance;
            continue;
        }
        if (c == '\t') {
            cur_x += font->cdata[0].xadvance * 4.0f;
            continue;
        }

        unsigned char uc = (unsigned char)c;
        int char_idx = (int)uc - FIRST_CHAR;
        if (char_idx < 0 || char_idx >= NUM_CHARS) {
            cur_x += font->size * 0.5f;
            continue;
        }

        stbtt_aligned_quad q;
        stbtt_GetBakedQuad(font->cdata, ATLAS_WIDTH, ATLAS_HEIGHT, (int)uc - FIRST_CHAR, &cur_x, &baseline_y, &q, 1);

        if (q.x1 > q.x0 && q.y1 > q.y0) {
            Rect dst = rect_make(q.x0, q.y0, q.x1 - q.x0, q.y1 - q.y0);
            Rect uv = rect_make(q.s0, q.t0, q.s1 - q.s0, q.t1 - q.t0);
            r_draw_textured_quad(dst, uv, font->texture_id, color);
        }
    }
}

Font* font_load_default_ui(float pixel_height) {
    const char* candidates[] = {
        "/usr/share/fonts/adwaita-sans-fonts/AdwaitaSans-Regular.ttf",
        "/usr/share/fonts/redhat/RedHatText-Regular.otf",
        "/usr/share/fonts/open-sans/OpenSans-Regular.ttf",
        "/usr/share/fonts/google-droid-sans-fonts/DroidSans.ttf",
        "/usr/share/fonts/google-noto/NotoSans-Light.ttf",
        NULL
    };
    for (int i = 0; candidates[i]; i++) {
        Font* f = font_load(candidates[i], pixel_height);
        if (f) return f;
    }
    return NULL;
}

Font* font_load_default_mono(float pixel_height) {
    const char* candidates[] = {
        "/usr/share/fonts/source-foundry-hack-fonts/Hack-Regular.ttf",
        "/usr/share/fonts/jetbrains-mono-nerd-fonts/JetBrainsMonoNerdFont-Regular.ttf",
        "/usr/share/fonts/urw-base35/NimbusMonoPS-Regular.otf",
        NULL
    };
    for (int i = 0; candidates[i]; i++) {
        Font* f = font_load(candidates[i], pixel_height);
        if (f) return f;
    }
    return font_load_default_ui(pixel_height);
}

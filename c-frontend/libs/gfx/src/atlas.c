#include "gfx/atlas.h"
#include <GL/gl.h>

#define ATLAS_GUTTER 1
#define ATLAS_MAX_SHELVES 256

typedef struct {
    uint16_t y;       // top of the shelf
    uint16_t h;       // shelf height
    uint16_t cursor;  // next free x
} Shelf;

static GLuint  s_texture = 0;
static int     s_size    = 0;
static Shelf   s_shelves[ATLAS_MAX_SHELVES];
static int     s_shelf_count = 0;
static uint16_t s_next_y = 0;
static size_t  s_area_used = 0;

// Scratch row buffer so a whole glyph can be uploaded in one glTexSubImage2D
// call instead of one call per row.
static uint8_t* s_staging = NULL;
static size_t   s_staging_cap = 0;

bool atlas_init(int size) {
    if (s_texture) return true;
    if (size <= 0 || (size & (size - 1)) != 0) return false;

    s_size = size;
    s_shelf_count = 0;
    s_next_y = 0;
    s_area_used = 0;

    // Zeroed so unallocated space samples as fully transparent.
    uint8_t* zeros = calloc(1, (size_t)size * (size_t)size);
    if (!zeros) return false;

    glGenTextures(1, &s_texture);
    glBindTexture(GL_TEXTURE_2D, s_texture);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    // No mipmaps: glyphs and icons are rasterized at their display size, and
    // mip levels would average across neighbouring atlas cells.
    glTexImage2D(GL_TEXTURE_2D, 0, GL_R8, size, size, 0, GL_RED, GL_UNSIGNED_BYTE, zeros);
    free(zeros);

    return glGetError() == GL_NO_ERROR;
}

void atlas_shutdown(void) {
    if (s_texture) {
        glDeleteTextures(1, &s_texture);
        s_texture = 0;
    }
    free(s_staging);
    s_staging = NULL;
    s_staging_cap = 0;
    s_size = 0;
    s_shelf_count = 0;
    s_next_y = 0;
    s_area_used = 0;
}

AtlasRegion atlas_alloc(int w, int h) {
    AtlasRegion out = {0};
    if (!s_texture || w <= 0 || h <= 0) return out;

    int aw = w + ATLAS_GUTTER;
    int ah = h + ATLAS_GUTTER;
    if (aw > s_size || ah > s_size) return out;

    // Best-fit among existing shelves: smallest shelf that still fits, so a
    // short glyph does not consume space on a tall shelf.
    Shelf* best = NULL;
    for (int i = 0; i < s_shelf_count; i++) {
        Shelf* s = &s_shelves[i];
        if (s->h < ah) continue;
        if (s->cursor + aw > s_size) continue;
        if (!best || s->h < best->h) best = s;
    }

    if (!best) {
        if (s_shelf_count >= ATLAS_MAX_SHELVES) return out;
        if (s_next_y + ah > s_size) return out;
        best = &s_shelves[s_shelf_count++];
        best->y = s_next_y;
        best->h = (uint16_t)ah;
        best->cursor = 0;
        s_next_y = (uint16_t)(s_next_y + ah);
    }

    out.x = best->cursor;
    out.y = best->y;
    out.w = (uint16_t)w;
    out.h = (uint16_t)h;
    best->cursor = (uint16_t)(best->cursor + aw);

    float inv = 1.0f / (float)s_size;
    out.u0 = (float)out.x * inv;
    out.v0 = (float)out.y * inv;
    out.u1 = (float)(out.x + out.w) * inv;
    out.v1 = (float)(out.y + out.h) * inv;
    out.valid = true;

    s_area_used += (size_t)aw * (size_t)ah;
    return out;
}

void atlas_upload(const AtlasRegion* region, const uint8_t* pixels, int src_stride) {
    if (!s_texture || !region || !region->valid || !pixels) return;
    if (region->w == 0 || region->h == 0) return;

    int w = region->w;
    int h = region->h;
    if (src_stride <= 0) src_stride = w;

    const uint8_t* upload_src = pixels;

    // GL_UNPACK_ROW_LENGTH lets a strided source upload directly. Only fall
    // back to compaction if the stride is negative-going or unusable.
    if (src_stride != w) {
        size_t need = (size_t)w * (size_t)h;
        if (need > s_staging_cap) {
            uint8_t* grown = realloc(s_staging, need);
            if (!grown) return;
            s_staging = grown;
            s_staging_cap = need;
        }
        for (int row = 0; row < h; row++) {
            memcpy(s_staging + (size_t)row * w, pixels + (size_t)row * src_stride, (size_t)w);
        }
        upload_src = s_staging;
    }

    glBindTexture(GL_TEXTURE_2D, s_texture);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
    glTexSubImage2D(GL_TEXTURE_2D, 0, region->x, region->y, w, h,
                    GL_RED, GL_UNSIGNED_BYTE, upload_src);
}

uint32_t atlas_texture_id(void) { return (uint32_t)s_texture; }
int      atlas_size(void)       { return s_size; }

float atlas_occupancy(void) {
    if (s_size <= 0) return 0.0f;
    return (float)s_area_used / ((float)s_size * (float)s_size);
}

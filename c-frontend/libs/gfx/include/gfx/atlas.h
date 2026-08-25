#ifndef GFX_ATLAS_H
#define GFX_ATLAS_H

#include "base/base.h"

// ---------------------------------------------------------------------------
// Shared single-channel (R8) texture atlas.
//
// Glyphs AND icons allocate from ONE texture on purpose. The renderer must
// flush its vertex batch whenever the bound texture changes, so a font atlas
// separate from an icon atlas forces a flush on every icon+label button. One
// atlas keeps the whole UI in a single batch.
//
// R8 rather than RGBA because both glyphs and vector icons are alpha masks:
// the colour comes from the vertex tint. That is 4x less memory and 4x less
// upload bandwidth.
//
// Packing is shelf-based (rows of uniform height). For monotonically-arriving
// glyph and icon boxes this wastes little and costs O(1) per allocation.
// ---------------------------------------------------------------------------

typedef struct {
    uint16_t x, y, w, h;   // pixel rect inside the atlas
    float u0, v0, u1, v1;  // normalized texture coordinates
    bool   valid;
} AtlasRegion;

// Creates the atlas texture. `size` must be a power of two (2048 recommended).
bool  atlas_init(int size);
void  atlas_shutdown(void);

// Reserves `w` x `h` pixels (a 1px transparent gutter is added automatically to
// stop bilinear sampling from bleeding neighbours in). Returns an invalid
// region when full.
AtlasRegion atlas_alloc(int w, int h);

// Uploads `w*h` single-channel pixels into a previously reserved region.
// `src_stride` is the byte stride of `pixels` (0 means tightly packed).
void  atlas_upload(const AtlasRegion* region, const uint8_t* pixels, int src_stride);

uint32_t atlas_texture_id(void);
int      atlas_size(void);

// Diagnostics: fraction of atlas area handed out so far, 0..1.
float atlas_occupancy(void);

#endif // GFX_ATLAS_H

#ifndef GL_BACKEND_H
#define GL_BACKEND_H

#include "base/base.h"

// Renderer tunables. Nothing here is hardcoded in the library; the application
// supplies values (see app/config.c) and may change them at runtime.
typedef struct {
    // Coverage gamma for alpha-mask quads (text and icons). Blending 8-bit
    // coverage directly in sRGB space makes light-on-dark glyphs read thin and
    // fuzzy; raising coverage to 1/gamma compensates. 1.0 disables it.
    float alpha_gamma;
    // Extra coverage added to partially-covered pixels, which thickens stems
    // that would otherwise wash out at small sizes. 0 disables it.
    float stem_darken;
    bool  multisample;
} GfxConfig;

GfxConfig gfx_config_default(void);
void      gfx_set_config(GfxConfig cfg);
GfxConfig gfx_get_config(void);

bool gl_backend_init(int logical_w, int logical_h, int fb_w, int fb_h);
void gl_backend_shutdown(void);
void gl_backend_resize(int logical_w, int logical_h, int fb_w, int fb_h);
void gl_backend_begin_frame(void);
void gl_backend_end_frame(void);

// Clipping stack
void r_push_clip(Rect rect);
void r_pop_clip(void);
Rect r_get_clip(void);
// Current clip-stack depth; nonzero at end of frame means an unbalanced push.
int  r_clip_depth(void);

// Drawing primitives
void r_draw_rect(Rect rect, Color color);
void r_draw_rect_rounded(Rect rect, float radius, Color color);
void r_draw_rect_bordered(Rect rect, float border_width, Color border_color, Color fill_color);
void r_draw_rect_rounded_bordered(Rect rect, float radius, float border_width, Color border_color, Color fill_color);

// Strokes a rounded-rect outline only, leaving the interior untouched. Use this
// for a border over content that is already drawn.
void r_draw_rect_rounded_outline(Rect rect, float radius, float border_width, Color color);
void r_draw_line(Vec2 p1, Vec2 p2, float thickness, Color color);

// Texture & Font Quad Drawing
void r_draw_textured_quad(Rect dst_rect, Rect src_uv, uint32_t texture_id, Color tint);

// Single-channel (R8) coverage texture: RGB comes from `tint`, alpha is
// tint.a * texel.r. Used for the shared glyph/icon atlas.
void r_draw_alpha_quad(Rect dst_rect, Rect src_uv, uint32_t texture_id, Color tint);

// Reads the framebuffer back and writes a binary PPM at native resolution.
// Exists so render quality can be inspected at real pixels: a compositor that
// scales the window makes any screen capture useless for judging sharpness.
bool r_capture_ppm(const char* path);

#endif // GL_BACKEND_H

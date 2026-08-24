#ifndef CLAY_RENDERER_H
#define CLAY_RENDERER_H

#include "silvapi.h"
#include "gl_backend.h"
#include "font.h"
#include "clay.h"

// Initialize Clay layout engine arena & text measurement callbacks
void clay_renderer_init(int window_w, int window_h);
void clay_renderer_shutdown(void);

// Layout frame lifecycle
void clay_renderer_begin_frame(int window_w, int window_h, float dt);
void clay_renderer_end_and_render(float dt);

// Helper to convert Silvapi Color to Clay_Color
static inline Clay_Color clay_color_from_rgba(Color c) {
    return (Clay_Color){
        .r = (float)(c.r * 255.0f),
        .g = (float)(c.g * 255.0f),
        .b = (float)(c.b * 255.0f),
        .a = (float)(c.a * 255.0f)
    };
}

// Helper to convert Clay_Color to Silvapi Color
static inline Color color_from_clay(Clay_Color c) {
    return (Color){
        .r = c.r / 255.0f,
        .g = c.g / 255.0f,
        .b = c.b / 255.0f,
        .a = c.a / 255.0f
    };
}

#endif // CLAY_RENDERER_H

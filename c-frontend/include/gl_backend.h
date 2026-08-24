#ifndef GL_BACKEND_H
#define GL_BACKEND_H

#include "silvapi.h"

typedef enum {
    ICON_NONE = 0,
    ICON_SEARCH,
    ICON_PLUS,
    ICON_CLOSE,
    ICON_PLAY,
    ICON_STOP,
    ICON_CHEVRON_RIGHT,
    ICON_CHEVRON_DOWN,
    ICON_FOLDER,
    ICON_FOLDER_OPEN,
    ICON_FILE,
    ICON_TRASH,
    ICON_COPY,
    ICON_CHECK,
    ICON_SETTINGS,
    ICON_VARIABLE,
    ICON_IMPORT,
    ICON_DOT,
    ICON_REFRESH,
    ICON_EXPAND,
    ICON_COLLAPSE,
    ICON_MORE,
    ICON_COUNT
} IconType;

bool gl_backend_init(int window_width, int window_height);
void gl_backend_shutdown(void);
void gl_backend_resize(int width, int height);
void gl_backend_begin_frame(void);
void gl_backend_end_frame(void);

// Clipping stack
void r_push_clip(Rect rect);
void r_pop_clip(void);
Rect r_get_clip(void);

// Drawing primitives
void r_draw_rect(Rect rect, Color color);
void r_draw_rect_rounded(Rect rect, float radius, Color color);
void r_draw_rect_bordered(Rect rect, float border_width, Color border_color, Color fill_color);
void r_draw_rect_rounded_bordered(Rect rect, float radius, float border_width, Color border_color, Color fill_color);
void r_draw_line(Vec2 p1, Vec2 p2, float thickness, Color color);

// Texture & Font Quad Drawing
void r_draw_textured_quad(Rect dst_rect, Rect src_uv, uint32_t texture_id, Color tint);

// Icons
void r_draw_icon(IconType icon, Rect rect, Color color);

#endif // GL_BACKEND_H

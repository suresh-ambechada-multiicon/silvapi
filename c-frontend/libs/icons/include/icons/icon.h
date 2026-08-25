#ifndef ICONS_ICON_H
#define ICONS_ICON_H

#include "base/base.h"
#include "gfx/gfx.h"

typedef enum {
    ICON_NONE = 0,
    ICON_SEARCH,
    ICON_PLUS,
    ICON_CLOSE,
    ICON_PLAY,
    ICON_STOP,
    ICON_SEND,
    ICON_CHEVRON_RIGHT,
    ICON_CHEVRON_DOWN,
    ICON_CHEVRON_LEFT,
    ICON_CHEVRON_UP,
    ICON_FOLDER,
    ICON_FOLDER_OPEN,
    ICON_FILE,
    ICON_FILE_CODE,
    ICON_TRASH,
    ICON_COPY,
    ICON_CHECK,
    ICON_SETTINGS,
    ICON_VARIABLE,
    ICON_IMPORT,
    ICON_EXPORT,
    ICON_DOT,
    ICON_REFRESH,
    ICON_HISTORY,
    ICON_EXPAND,
    ICON_COLLAPSE,
    ICON_MORE,
    ICON_GLOBE,
    ICON_CLOCK,
    ICON_LOCK,
    ICON_KEY,
    ICON_DATABASE,
    ICON_TABLE,
    ICON_FILTER,
    ICON_EYE,
    ICON_EYE_OFF,
    ICON_TERMINAL,
    ICON_LAYERS,
    ICON_SPARKLES,
    ICON_BOLT,
    ICON_SHIELD,
    ICON_CODE,
    ICON_INFO,
    ICON_CHECK_CIRCLE,
    ICON_ALERT_TRIANGLE,
    ICON_COUNT
} IconType;

// ---------------------------------------------------------------------------
// Vector icon system.
//
// Icons are rasterized on demand at the exact pixel size they will be drawn
// at, and cached per (icon, pixel size). The previous implementation
// rasterized every icon once at 52px into a fixed 64px grid cell, so a 14px
// icon was a 52 -> 14 downscale: a 2-unit stroke on a 24-unit viewBox came out
// under one pixel wide and washed out, and the 6px cell margin meant the quad
// only ever showed 52/64 of its area, rendering every icon 19% small.
// ---------------------------------------------------------------------------

// Icon tunables. Supplied by the application; nothing is hardcoded here.
typedef struct {
    // Stroke width in SVG viewBox units. The bundled icons are authored at 2.0
    // on a 24-unit viewBox; raising it thickens every icon uniformly, which is
    // what small icons need to read clearly against a dark background.
    float stroke_width;
    // Extra viewBox units of padding so a stroke centred on the viewBox edge is
    // not clipped in half. Scales with stroke_width when left at 0.
    float edge_padding;
    float dpi_scale;
} IconConfig;

IconConfig icon_config_default(void);
// Changing stroke width or DPI invalidates every cached raster.
void       icon_set_config(IconConfig cfg);
IconConfig icon_get_config(void);

void icon_system_init(void);
void icon_system_shutdown(void);

// Draws `icon` filling `rect`, tinted with `color`. `rect` is in logical
// pixels; pass the framebuffer scale so the raster matches physical pixels on
// HiDPI displays.
void icon_draw(IconType icon, Rect rect, Color color);
void icon_draw_scaled(IconType icon, Rect rect, Color color, float dpi_scale);

// Sets the scale applied by icon_draw(); call on window/DPI change.
void icon_set_dpi_scale(float dpi_scale);

// Number of distinct (icon, size) rasters currently cached.
size_t icon_cache_count(void);

#endif // ICONS_ICON_H

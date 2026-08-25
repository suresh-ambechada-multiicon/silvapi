#include "icons/icon.h"
#include "gfx/atlas.h"

#define NANOSVG_IMPLEMENTATION
#include "nanosvg.h"
#define NANOSVGRAST_IMPLEMENTATION
#include "nanosvgrast.h"

static const char* s_icon_svgs[ICON_COUNT] = {
    [ICON_NONE] = "",
    [ICON_SEARCH] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><circle cx=\"11\" cy=\"11\" r=\"8\"/><line x1=\"21\" y1=\"21\" x2=\"16.65\" y2=\"16.65\"/></svg>",
    [ICON_PLUS] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><line x1=\"12\" y1=\"5\" x2=\"12\" y2=\"19\"/><line x1=\"5\" y1=\"12\" x2=\"19\" y2=\"12\"/></svg>",
    [ICON_CLOSE] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><line x1=\"18\" y1=\"6\" x2=\"6\" y2=\"18\"/><line x1=\"6\" y1=\"6\" x2=\"18\" y2=\"18\"/></svg>",
    [ICON_PLAY] = "<svg viewBox=\"0 0 24 24\" fill=\"white\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><polygon points=\"5 3 19 12 5 21 5 3\"/></svg>",
    [ICON_STOP] = "<svg viewBox=\"0 0 24 24\" fill=\"white\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><rect x=\"4\" y=\"4\" width=\"16\" height=\"16\" rx=\"2\"/></svg>",
    [ICON_SEND] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><line x1=\"22\" y1=\"2\" x2=\"11\" y2=\"13\"/><polygon points=\"22 2 15 22 11 13 2 9 22 2\"/></svg>",
    [ICON_CHEVRON_RIGHT] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><polyline points=\"9 18 15 12 9 6\"/></svg>",
    [ICON_CHEVRON_DOWN] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><polyline points=\"6 9 12 15 18 9\"/></svg>",
    [ICON_CHEVRON_LEFT] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><polyline points=\"15 18 9 12 15 6\"/></svg>",
    [ICON_CHEVRON_UP] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><polyline points=\"18 15 12 9 6 15\"/></svg>",
    [ICON_FOLDER] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z\"/></svg>",
    [ICON_FOLDER_OPEN] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z\"/><line x1=\"2\" y1=\"12\" x2=\"22\" y2=\"12\"/></svg>",
    [ICON_FILE] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z\"/><polyline points=\"13 2 13 9 20 9\"/></svg>",
    [ICON_FILE_CODE] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z\"/><polyline points=\"14 2 14 8 20 8\"/><polyline points=\"10 13 8 15 10 17\"/><polyline points=\"14 13 16 15 14 17\"/></svg>",
    [ICON_TRASH] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><polyline points=\"3 6 5 6 21 6\"/><path d=\"M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2\"/><line x1=\"10\" y1=\"11\" x2=\"10\" y2=\"17\"/><line x1=\"14\" y1=\"11\" x2=\"14\" y2=\"17\"/></svg>",
    [ICON_COPY] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><rect x=\"9\" y=\"9\" width=\"13\" height=\"13\" rx=\"2\" ry=\"2\"/><path d=\"M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1\"/></svg>",
    [ICON_CHECK] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><polyline points=\"20 6 9 17 4 12\"/></svg>",
    [ICON_SETTINGS] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><circle cx=\"12\" cy=\"12\" r=\"3\"/><path d=\"M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z\"/></svg>",
    [ICON_VARIABLE] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M4 7c0-2.2 1.8-4 4-4s4 1.8 4 4v10c0 2.2 1.8 4 4 4s4-1.8 4-4\"/><line x1=\"9\" y1=\"12\" x2=\"15\" y2=\"12\"/></svg>",
    [ICON_IMPORT] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4\"/><polyline points=\"7 10 12 15 17 10\"/><line x1=\"12\" y1=\"15\" x2=\"12\" y2=\"3\"/></svg>",
    [ICON_EXPORT] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4\"/><polyline points=\"17 8 12 3 7 8\"/><line x1=\"12\" y1=\"3\" x2=\"12\" y2=\"15\"/></svg>",
    [ICON_DOT] = "<svg viewBox=\"0 0 24 24\" fill=\"white\" stroke=\"white\" stroke-width=\"1\"><circle cx=\"12\" cy=\"12\" r=\"5\"/></svg>",
    [ICON_REFRESH] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><polyline points=\"23 4 23 10 17 10\"/><polyline points=\"1 20 1 14 7 14\"/><path d=\"M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15\"/></svg>",
    [ICON_HISTORY] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><circle cx=\"12\" cy=\"12\" r=\"10\"/><polyline points=\"12 6 12 12 16 14\"/></svg>",
    [ICON_EXPAND] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><rect x=\"3\" y=\"3\" width=\"18\" height=\"18\" rx=\"2\" ry=\"2\"/><line x1=\"9\" y1=\"3\" x2=\"9\" y2=\"21\"/><polyline points=\"14 9 17 12 14 15\"/></svg>",
    [ICON_COLLAPSE] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><rect x=\"3\" y=\"3\" width=\"18\" height=\"18\" rx=\"2\" ry=\"2\"/><line x1=\"9\" y1=\"3\" x2=\"9\" y2=\"21\"/><polyline points=\"16 15 13 12 16 9\"/></svg>",
    [ICON_MORE] = "<svg viewBox=\"0 0 24 24\" fill=\"white\" stroke=\"white\" stroke-width=\"1\"><circle cx=\"12\" cy=\"12\" r=\"2\"/><circle cx=\"19\" cy=\"12\" r=\"2\"/><circle cx=\"5\" cy=\"12\" r=\"2\"/></svg>",
    [ICON_GLOBE] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><circle cx=\"12\" cy=\"12\" r=\"10\"/><line x1=\"2\" y1=\"12\" x2=\"22\" y2=\"12\"/><path d=\"M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z\"/></svg>",
    [ICON_CLOCK] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><circle cx=\"12\" cy=\"12\" r=\"10\"/><polyline points=\"12 6 12 12 16 14\"/></svg>",
    [ICON_LOCK] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><rect x=\"3\" y=\"11\" width=\"18\" height=\"11\" rx=\"2\" ry=\"2\"/><path d=\"M7 11V7a5 5 0 0 1 10 0v4\"/></svg>",
    [ICON_KEY] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M21 2l-2 2m-1.5 1.5L14 9l-1.5-1.5-2 2 1.5 1.5L2 21l3 1 1-3 2-2 1.5 1.5 2-2-1.5-1.5 3.5-3.5a7 7 0 1 0-4.5-4.5\"/></svg>",
    [ICON_DATABASE] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><ellipse cx=\"12\" cy=\"5\" rx=\"9\" ry=\"3\"/><path d=\"M21 12c0 1.66-4 3-9 3s-9-1.34-9-3\"/><path d=\"M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5\"/></svg>",
    [ICON_TABLE] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><rect x=\"3\" y=\"3\" width=\"18\" height=\"18\" rx=\"2\" ry=\"2\"/><line x1=\"3\" y1=\"9\" x2=\"21\" y2=\"9\"/><line x1=\"3\" y1=\"15\" x2=\"21\" y2=\"15\"/><line x1=\"12\" y1=\"3\" x2=\"12\" y2=\"21\"/></svg>",
    [ICON_FILTER] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><polygon points=\"22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3\"/></svg>",
    [ICON_EYE] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z\"/><circle cx=\"12\" cy=\"12\" r=\"3\"/></svg>",
    [ICON_EYE_OFF] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24\"/><line x1=\"1\" y1=\"1\" x2=\"23\" y2=\"23\"/></svg>",
    [ICON_TERMINAL] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><polyline points=\"4 17 10 11 4 5\"/><line x1=\"12\" y1=\"19\" x2=\"20\" y2=\"19\"/></svg>",
    [ICON_LAYERS] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><polygon points=\"12 2 2 7 12 12 22 7 12 2\"/><polyline points=\"2 17 12 22 22 17\"/><polyline points=\"2 12 12 17 22 12\"/></svg>",
    [ICON_SPARKLES] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M12 2v4m0 12v4M4.93 4.93l2.83 2.83m8.48 8.48l2.83 2.83M2 12h4m12 0h4M4.93 19.07l2.83-2.83m8.48-8.48l2.83-2.83\"/></svg>",
    [ICON_BOLT] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><polygon points=\"13 2 3 14 12 14 11 22 21 10 12 10 13 2\"/></svg>",
    [ICON_SHIELD] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z\"/></svg>",
    [ICON_CODE] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><polyline points=\"16 18 22 12 16 6\"/><polyline points=\"8 6 2 12 8 18\"/></svg>",
    [ICON_INFO] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><circle cx=\"12\" cy=\"12\" r=\"10\"/><line x1=\"12\" y1=\"16\" x2=\"12\" y2=\"12\"/><line x1=\"12\" y1=\"8\" x2=\"12.01\" y2=\"8\"/></svg>",
    [ICON_CHECK_CIRCLE] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M22 11.08V12a10 10 0 1 1-5.93-9.14\"/><polyline points=\"22 4 12 14.01 9 11.01\"/></svg>",
    [ICON_ALERT_TRIANGLE] = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z\"/><line x1=\"12\" y1=\"9\" x2=\"12\" y2=\"13\"/><line x1=\"12\" y1=\"17\" x2=\"12.01\" y2=\"17\"/></svg>",
};

// ---------------------------------------------------------------------------
// Per-(icon, size) raster cache backed by the shared R8 atlas.
// ---------------------------------------------------------------------------

#define ICON_CACHE_SLOTS 512
#define ICON_MAX_PX      256

typedef struct {
    uint16_t    icon;
    uint16_t    px;
    AtlasRegion region;
    bool        used;
} IconRaster;

static IconRaster s_cache[ICON_CACHE_SLOTS];
static size_t     s_cache_count = 0;
static struct NSVGrasterizer* s_rast = NULL;
static IconConfig s_cfg = { .stroke_width = 2.0f, .edge_padding = 0.0f, .dpi_scale = 1.0f };
static bool       s_initialized = false;

IconConfig icon_config_default(void) {
    IconConfig c = {0};
    // Lucide source stroke is 2.0; 2.25 reads noticeably better at 14-16px
    // without looking heavy at larger sizes.
    c.stroke_width = 2.25f;
    c.edge_padding = 0.0f;   // derived from stroke_width when zero
    c.dpi_scale = 1.0f;
    return c;
}

IconConfig icon_get_config(void) { return s_cfg; }

// Parsed SVG trees are kept so re-rasterizing at a new size does not re-parse.
static struct NSVGimage* s_parsed[ICON_COUNT];

void icon_system_init(void) {
    if (s_initialized) return;
    s_rast = nsvgCreateRasterizer();
    if (!s_rast) {
        fprintf(stderr, "[Icons] Failed to create rasterizer\n");
        return;
    }
    memset(s_cache, 0, sizeof(s_cache));
    memset(s_parsed, 0, sizeof(s_parsed));
    s_cache_count = 0;
    if (s_cfg.stroke_width <= 0.0f) s_cfg = icon_config_default();
    s_initialized = true;
}

void icon_system_shutdown(void) {
    for (int i = 0; i < ICON_COUNT; i++) {
        if (s_parsed[i]) {
            nsvgDelete(s_parsed[i]);
            s_parsed[i] = NULL;
        }
    }
    if (s_rast) {
        nsvgDeleteRasterizer(s_rast);
        s_rast = NULL;
    }
    memset(s_cache, 0, sizeof(s_cache));
    s_cache_count = 0;
    s_initialized = false;
}

// Drops every cached raster; the atlas regions they held are not reclaimed, so
// this must not be called per frame.
static void icon_cache_clear(void) {
    memset(s_cache, 0, sizeof(s_cache));
    s_cache_count = 0;
}

void icon_set_config(IconConfig cfg) {
    if (cfg.stroke_width <= 0.0f) cfg.stroke_width = 2.0f;
    if (cfg.dpi_scale <= 0.0f)    cfg.dpi_scale = 1.0f;

    bool raster_changed = (cfg.stroke_width != s_cfg.stroke_width) ||
                          (cfg.edge_padding != s_cfg.edge_padding) ||
                          (cfg.dpi_scale != s_cfg.dpi_scale);
    s_cfg = cfg;
    if (raster_changed) icon_cache_clear();
}

void icon_set_dpi_scale(float dpi_scale) {
    if (dpi_scale <= 0.0f || dpi_scale == s_cfg.dpi_scale) return;
    IconConfig c = s_cfg;
    c.dpi_scale = dpi_scale;
    icon_set_config(c);
}

size_t icon_cache_count(void) { return s_cache_count; }

static struct NSVGimage* icon_parsed(IconType icon) {
    if (icon <= ICON_NONE || icon >= ICON_COUNT) return NULL;
    if (s_parsed[icon]) return s_parsed[icon];

    const char* svg_src = s_icon_svgs[icon];
    if (!svg_src || !*svg_src) return NULL;

    // nsvgParse mutates its input, so it gets a copy.
    char* copy = strdup(svg_src);
    if (!copy) return NULL;
    s_parsed[icon] = nsvgParse(copy, "px", 96.0f);
    free(copy);
    return s_parsed[icon];
}

static const IconRaster* icon_raster(IconType icon, int px) {
    if (!s_initialized || icon <= ICON_NONE || icon >= ICON_COUNT) return NULL;
    if (px <= 0) return NULL;
    if (px > ICON_MAX_PX) px = ICON_MAX_PX;

    uint32_t hash = ((uint32_t)icon * 2654435761u) ^ ((uint32_t)px * 2246822519u);
    for (int probe = 0; probe < 32; probe++) {
        IconRaster* slot = &s_cache[(hash + (uint32_t)probe) % ICON_CACHE_SLOTS];

        if (slot->used) {
            if (slot->icon == (uint16_t)icon && slot->px == (uint16_t)px) return slot;
            continue;
        }

        struct NSVGimage* image = icon_parsed(icon);
        if (!image) return NULL;

        // Stroke width is applied to the parsed shape tree rather than by
        // rewriting the SVG text, so re-rasterizing at a new width costs no
        // re-parse. Shapes authored as fills are left alone.
        for (struct NSVGshape* shape = image->shapes; shape; shape = shape->next) {
            if (shape->stroke.type != NSVG_PAINT_NONE) {
                shape->strokeWidth = s_cfg.stroke_width;
            }
        }

        float src_w = image->width  > 0.0f ? image->width  : 24.0f;
        float src_h = image->height > 0.0f ? image->height : 24.0f;

        // Rasterize at the full requested size: no interior margin, so the quad
        // shows the icon at exactly the size the caller asked for.
        //
        // A stroke centred on the viewBox edge would be clipped in half, so the
        // raster is inset by half the widest stroke and the geometry is scaled
        // to match. Lucide icons use stroke-width 2 on a 24 viewBox.
        // Half the stroke sticks out past the viewBox edge, so inset by that
        // much (plus any configured extra) and scale the geometry to match.
        float stroke_pad_units = s_cfg.edge_padding > 0.0f
                                     ? s_cfg.edge_padding
                                     : s_cfg.stroke_width * 0.5f;
        float scale = (float)px / (src_w + stroke_pad_units * 2.0f);
        float offset = stroke_pad_units * scale;

        int bmp_w = px;
        int bmp_h = (int)(src_h * scale + offset * 2.0f + 0.5f);
        if (bmp_h <= 0) bmp_h = px;

        // nanosvg only writes RGBA, so rasterize to RGBA then compact the alpha
        // channel down to the R8 the atlas stores.
        uint8_t* rgba = calloc(1, (size_t)bmp_w * (size_t)bmp_h * 4);
        if (!rgba) return NULL;

        nsvgRasterize(s_rast, image, offset, offset, scale,
                      rgba, bmp_w, bmp_h, bmp_w * 4);

        uint8_t* alpha = malloc((size_t)bmp_w * (size_t)bmp_h);
        if (!alpha) { free(rgba); return NULL; }
        for (size_t i = 0, n = (size_t)bmp_w * (size_t)bmp_h; i < n; i++) {
            alpha[i] = rgba[i * 4 + 3];
        }
        free(rgba);

        AtlasRegion region = atlas_alloc(bmp_w, bmp_h);
        if (!region.valid) {
            fprintf(stderr, "[Icons] Atlas full (occupancy %.2f), icon %d @ %dpx dropped\n",
                    (double)atlas_occupancy(), (int)icon, px);
            free(alpha);
            return NULL;
        }
        atlas_upload(&region, alpha, bmp_w);
        free(alpha);

        slot->icon   = (uint16_t)icon;
        slot->px     = (uint16_t)px;
        slot->region = region;
        slot->used   = true;
        s_cache_count++;
        return slot;
    }
    return NULL;
}

void icon_draw_scaled(IconType icon, Rect rect, Color color, float dpi_scale) {
    if (icon <= ICON_NONE || icon >= ICON_COUNT) return;
    if (rect.w <= 0.0f || rect.h <= 0.0f) return;

    float side = rect.w < rect.h ? rect.w : rect.h;
    int px = (int)(side * (dpi_scale > 0.0f ? dpi_scale : 1.0f) + 0.5f);
    if (px <= 0) return;

    const IconRaster* raster = icon_raster(icon, px);
    if (!raster) return;

    // Snap to whole pixels: a vector icon sampled at a fractional offset loses
    // the crispness it was just rasterized at.
    float draw_w = (float)raster->region.w / (dpi_scale > 0.0f ? dpi_scale : 1.0f);
    float draw_h = (float)raster->region.h / (dpi_scale > 0.0f ? dpi_scale : 1.0f);
    float dx = floorf(rect.x + (rect.w - draw_w) * 0.5f + 0.5f);
    float dy = floorf(rect.y + (rect.h - draw_h) * 0.5f + 0.5f);

    Rect dst = rect_make(dx, dy, draw_w, draw_h);
    Rect uv  = rect_make(raster->region.u0, raster->region.v0,
                         raster->region.u1 - raster->region.u0,
                         raster->region.v1 - raster->region.v0);

    r_draw_alpha_quad(dst, uv, atlas_texture_id(), color);
}

void icon_draw(IconType icon, Rect rect, Color color) {
    icon_draw_scaled(icon, rect, color, s_cfg.dpi_scale);
}

#ifndef APP_CONFIG_H
#define APP_CONFIG_H

#include "base/base.h"
#include "gfx/gfx.h"
#include "text/font.h"
#include "icons/icon.h"

// ---------------------------------------------------------------------------
// Application configuration.
//
// The libraries under libs/ hold no tunable constants: each exposes a config
// struct and a setter, and this is the only layer that knows about files or
// JSON. That keeps the libraries reusable by a frontend with a completely
// different configuration story.
//
// Loaded from, in order of preference:
//   $SILVAPI_CONFIG
//   $XDG_CONFIG_HOME/silvapi/config.json   (or ~/.config/silvapi/config.json)
//   ./silvapi.json
//
// Any missing key keeps its default, so a partial file is valid.
// ---------------------------------------------------------------------------

typedef struct {
    // Typography, in logical pixels.
    float font_size_ui;
    float font_size_mono;
    float font_size_bold;
    char  font_family_ui[128];    // empty = fontconfig's sans-serif default
    char  font_family_mono[128];  // empty = fontconfig's monospace default

    // Rendering. dpi_scale <= 0 means "follow the window's content scale".
    float dpi_scale;

    GfxConfig  gfx;
    TextConfig text;
    IconConfig icon;

    // Layout metrics, so the views stop carrying magic numbers.
    float topbar_height;
    float row_height;
    float corner_radius;
    float pad_small;
    float pad_medium;
    float pad_large;
    float gap_small;
    float gap_medium;
    float icon_size_small;
    float icon_size_medium;

    char  theme_name[64];
    char  source_path[512];   // file actually loaded, empty if defaults only
} AppConfig;

// Defaults only; never fails.
AppConfig app_config_default(void);

// Loads and merges over the defaults. Returns false if no file was found, in
// which case the defaults are still written to `out`.
bool app_config_load(AppConfig* out);

// Pushes the config into every library. Safe to call again after an edit or a
// DPI change; the libraries invalidate their own caches as needed.
void app_config_apply(const AppConfig* cfg);

// Writes the effective config, so a user has a complete file to edit.
bool app_config_write(const AppConfig* cfg, const char* path);

// The live config.
extern AppConfig g_config;

#endif // APP_CONFIG_H

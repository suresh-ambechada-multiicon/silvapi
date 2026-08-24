#ifndef THEME_H
#define THEME_H

#include "silvapi.h"

typedef struct {
    const char* name;
    bool is_dark;

    // Window & Panels
    Color background;
    Color panel_bg;
    Color sidebar_bg;
    Color topbar_bg;
    Color tabbar_bg;
    Color tab_active_bg;
    Color tab_inactive_bg;
    Color border;
    Color border_subtle;

    // Text
    Color foreground;
    Color text_muted;
    Color text_dim;
    Color text_selected;

    // Accents & Buttons
    Color primary;
    Color primary_hover;
    Color primary_active;
    Color primary_text;
    
    Color secondary;
    Color secondary_hover;
    Color secondary_active;
    Color secondary_text;

    Color danger;
    Color danger_hover;
    Color success;
    Color warning;
    Color info;

    // Inputs
    Color input_bg;
    Color input_border;
    Color input_border_focus;
    Color input_placeholder;

    // Scrollbars & Selections
    Color scrollbar_bg;
    Color scrollbar_thumb;
    Color selection_bg;

    // HTTP Methods
    Color method_get;
    Color method_post;
    Color method_put;
    Color method_patch;
    Color method_delete;
    Color method_head;
    Color method_options;

    // Status codes
    Color status_2xx;
    Color status_3xx;
    Color status_4xx;
    Color status_5xx;

    // Syntax Highlighting (JSON, headers, HTTP)
    Color syntax_key;
    Color syntax_string;
    Color syntax_number;
    Color syntax_boolean;
    Color syntax_null;
    Color syntax_punctuation;
    Color syntax_comment;

} Theme;

typedef enum {
    THEME_AYU_DARK = 0,
    THEME_AYU_LIGHT,
    THEME_CATPPUCCIN,
    THEME_GRUVBOX,
    THEME_COUNT
} ThemePreset;

void theme_init(void);
Theme* theme_get_active(void);
void theme_set_preset(ThemePreset preset);
const char* theme_get_preset_name(ThemePreset preset);
Color theme_get_method_color(const char* method);
Color theme_get_status_color(int status_code);

#endif // THEME_H

#include "theme.h"
#include <strings.h>

static Theme s_themes[THEME_COUNT];
static ThemePreset s_active_preset = THEME_AYU_DARK;

void theme_init(void) {
    // 1. Ayu Dark (matching uploaded screenshots & themes/ayu.json)
    s_themes[THEME_AYU_DARK] = (Theme){
        .name = "Ayu Dark",
        .is_dark = true,

        .background = color_hex(0x0D1016),
        .panel_bg = color_hex(0x16191F),
        .sidebar_bg = color_hex(0x16191F),
        .topbar_bg = color_hex(0x16191F),
        .tabbar_bg = color_hex(0x16191F),
        .tab_active_bg = color_hex(0x0D1016),
        .tab_inactive_bg = color_hex(0x16191F),
        .border = color_hex(0x292A2C),
        .border_subtle = color_hex(0x1F2127),

        .foreground = color_hex(0xB3B1AD),
        .text_muted = color_hex(0x696A6A),
        .text_dim = color_hex(0x4B4C4E),
        .text_selected = color_hex(0xFFFFFF),

        .primary = color_hex(0x5AC1FE),
        .primary_hover = color_hex(0x3DAEE9),
        .primary_active = color_hex(0x36A3D9),
        .primary_text = color_hex(0x1F2430),

        .secondary = color_hex(0x1F2127),
        .secondary_hover = color_hex(0x26282F),
        .secondary_active = color_hex(0x2D2F34),
        .secondary_text = color_hex(0xB3B1AD),

        .danger = color_hex(0xEF7177),
        .danger_hover = color_hex(0xD85A60),
        .success = color_hex(0xAAD84C),
        .warning = color_hex(0xFEB454), 
        .info = color_hex(0x5AC1FE),

        .input_bg = color_hex(0x0D1016),
        .input_border = color_hex(0x2D2F34),
        .input_border_focus = color_hex(0x5AC1FE),
        .input_placeholder = color_hex(0x4B4C4E),

        .scrollbar_bg = color_rgba(0, 0, 0, 0),
        .scrollbar_thumb = color_rgba(0.75f, 0.74f, 0.71f, 0.25f),
        .selection_bg = color_rgba(0.35f, 0.76f, 1.0f, 0.25f),

        .method_get = color_hex(0x5AC1FE),
        .method_post = color_hex(0xFEB454),
        .method_put = color_hex(0xF29668),
        .method_patch = color_hex(0xD2A6FF),
        .method_delete = color_hex(0xEF7177),
        .method_head = color_hex(0x5AC1FE),
        .method_options = color_hex(0xA6A5A0),

        .status_2xx = color_hex(0xAAD84C),
        .status_3xx = color_hex(0xFEB454),
        .status_4xx = color_hex(0xEF7177),
        .status_5xx = color_hex(0xEF7177),

        .syntax_key = color_hex(0x5AC1FE),
        .syntax_string = color_hex(0xA9D94B),
        .syntax_number = color_hex(0xD2A6FF),
        .syntax_boolean = color_hex(0xD2A6FF),
        .syntax_null = color_hex(0xFEB454),
        .syntax_punctuation = color_hex(0xA6A5A0),
        .syntax_comment = color_hex(0x696A6A),
    };

    // 2. Ayu Light
    s_themes[THEME_AYU_LIGHT] = (Theme){
        .name = "Ayu Light",
        .is_dark = false,

        .background = color_hex(0xFCFCFC),
        .panel_bg = color_hex(0xF3F4F5),
        .sidebar_bg = color_hex(0xF4F4F5),
        .topbar_bg = color_hex(0xECECED),
        .tabbar_bg = color_hex(0xF4F4F5),
        .tab_active_bg = color_hex(0xFCFCFC),
        .tab_inactive_bg = color_hex(0xF4F4F5),
        .border = color_hex(0xCFD1D2),
        .border_subtle = color_hex(0xE6E6E6),
       
        .foreground = color_hex(0x5C6166),
        .text_muted = color_hex(0x99A0A6),
        .text_dim = color_hex(0xABB0B6),
        .text_selected = color_hex(0x000000),

        .primary = color_hex(0x55B4D3),
        .primary_hover = color_hex(0x5FB9D7),
        .primary_active = color_hex(0x42ACCF),
        .primary_text = color_hex(0xFAFAFA),

        .secondary = color_hex(0xECECED),
        .secondary_hover = color_hex(0xE0E0E0),
        .secondary_active = color_hex(0xCECECE),
        .secondary_text = color_hex(0x5C6773),

        .danger = color_hex(0xEF7271),
        .danger_hover = color_hex(0xD85A60),
        .success = color_hex(0x85B304),
        .warning = color_hex(0xF1AD49),
        .info = color_hex(0x3B9EE5),

        .input_bg = color_hex(0xFFFFFF),
        .input_border = color_hex(0xD9DCE3),
        .input_border_focus = color_hex(0x55B4D3),
        .input_placeholder = color_hex(0xABB0B6),

        .scrollbar_bg = color_rgba(0, 0, 0, 0),
        .scrollbar_thumb = color_rgba(0.36f, 0.38f, 0.40f, 0.25f),
        .selection_bg = color_rgba(0.33f, 0.70f, 0.83f, 0.20f),

        .method_get = color_hex(0x3B9EE5),
        .method_post = color_hex(0xF1AD49),
        .method_put = color_hex(0xFA8D3E),
        .method_patch = color_hex(0xA37ACC),
        .method_delete = color_hex(0xEF7271),
        .method_head = color_hex(0x3B9EE5),
        .method_options = color_hex(0x73777B),

        .status_2xx = color_hex(0x85B304),
        .status_3xx = color_hex(0xF1AD49),
        .status_4xx = color_hex(0xEF7271),
        .status_5xx = color_hex(0xEF7271),

        .syntax_key = color_hex(0x3B9EE5),
        .syntax_string = color_hex(0x86B300),
        .syntax_number = color_hex(0xA37ACC),
        .syntax_boolean = color_hex(0xA37ACC),
        .syntax_null = color_hex(0xF98D3F),
        .syntax_punctuation = color_hex(0x73777B),
        .syntax_comment = color_hex(0x898D90),
    };

    // 3. Catppuccin Mocha
    s_themes[THEME_CATPPUCCIN] = (Theme){
        .name = "Catppuccin Mocha",
        .is_dark = true,

        .background = color_hex(0x1E1E2E),
        .panel_bg = color_hex(0x181825),
        .sidebar_bg = color_hex(0x181825),
        .topbar_bg = color_hex(0x181825),
        .tabbar_bg = color_hex(0x181825),
        .tab_active_bg = color_hex(0x1E1E2E),
        .tab_inactive_bg = color_hex(0x181825),
        .border = color_hex(0x313244),
        .border_subtle = color_hex(0x27273A),

        .foreground = color_hex(0xCDD6F4),
        .text_muted = color_hex(0x7F849C),
        .text_dim = color_hex(0x585B70),
        .text_selected = color_hex(0xFFFFFF),

        .primary = color_hex(0x89B4FA),
        .primary_hover = color_hex(0xB4BEFE),
        .primary_active = color_hex(0x74C7EC),
        .primary_text = color_hex(0x11111B),

        .secondary = color_hex(0x313244),
        .secondary_hover = color_hex(0x45475A),
        .secondary_active = color_hex(0x585B70),
        .secondary_text = color_hex(0xCDD6F4),

        .danger = color_hex(0xF38BA8),
        .danger_hover = color_hex(0xEB6F92),
        .success = color_hex(0xA6E3A1),
        .warning = color_hex(0xF9E2AF),
        .info = color_hex(0x89DCEB),

        .input_bg = color_hex(0x181825),
        .input_border = color_hex(0x313244),
        .input_border_focus = color_hex(0x89B4FA),
        .input_placeholder = color_hex(0x585B70),

        .scrollbar_bg = color_rgba(0, 0, 0, 0),
        .scrollbar_thumb = color_rgba(0.5f, 0.52f, 0.61f, 0.25f),
        .selection_bg = color_rgba(0.54f, 0.71f, 0.98f, 0.25f),

        .method_get = color_hex(0x89B4FA),
        .method_post = color_hex(0xF9E2AF),
        .method_put = color_hex(0xFAB387),
        .method_patch = color_hex(0xCBA6F7),
        .method_delete = color_hex(0xF38BA8),
        .method_head = color_hex(0x89DCEB),
        .method_options = color_hex(0x9399B2),

        .status_2xx = color_hex(0xA6E3A1),
        .status_3xx = color_hex(0xF9E2AF),
        .status_4xx = color_hex(0xF38BA8),
        .status_5xx = color_hex(0xF38BA8),

        .syntax_key = color_hex(0x89B4FA),
        .syntax_string = color_hex(0xA6E3A1),
        .syntax_number = color_hex(0xFAB387),
        .syntax_boolean = color_hex(0xCBA6F7),
        .syntax_null = color_hex(0xF9E2AF),
        .syntax_punctuation = color_hex(0x9399B2),
        .syntax_comment = color_hex(0x6C7086),
    };

    // 4. Gruvbox Dark
    s_themes[THEME_GRUVBOX] = (Theme){
        .name = "Gruvbox Dark",
        .is_dark = true,

        .background = color_hex(0x282828),
        .panel_bg = color_hex(0x1D2021),
        .sidebar_bg = color_hex(0x1D2021),
        .topbar_bg = color_hex(0x1D2021),
        .tabbar_bg = color_hex(0x1D2021),
        .tab_active_bg = color_hex(0x282828),
        .tab_inactive_bg = color_hex(0x1D2021),
        .border = color_hex(0x3C3836),
        .border_subtle = color_hex(0x32302F),

        .foreground = color_hex(0xEBDBB2),
        .text_muted = color_hex(0x928374),
        .text_dim = color_hex(0x665C54),
        .text_selected = color_hex(0xFFFFFF),

        .primary = color_hex(0x83A598),
        .primary_hover = color_hex(0x458588),
        .primary_active = color_hex(0x076678),
        .primary_text = color_hex(0x1D2021),

        .secondary = color_hex(0x3C3836),
        .secondary_hover = color_hex(0x504945),
        .secondary_active = color_hex(0x665C54),
        .secondary_text = color_hex(0xEBDBB2),

        .danger = color_hex(0xFB4934),
        .danger_hover = color_hex(0xCC241D),
        .success = color_hex(0xB8BB26),
        .warning = color_hex(0xFABD2F),
        .info = color_hex(0x83A598),

        .input_bg = color_hex(0x1D2021),
        .input_border = color_hex(0x3C3836),
        .input_border_focus = color_hex(0x83A598),
        .input_placeholder = color_hex(0x665C54),

        .scrollbar_bg = color_rgba(0, 0, 0, 0),
        .scrollbar_thumb = color_rgba(0.57f, 0.51f, 0.45f, 0.25f),
        .selection_bg = color_rgba(0.51f, 0.65f, 0.60f, 0.25f),

        .method_get = color_hex(0x83A598),
        .method_post = color_hex(0xFABD2F),
        .method_put = color_hex(0xFE8019),
        .method_patch = color_hex(0xD3869B),
        .method_delete = color_hex(0xFB4934),
        .method_head = color_hex(0x8EC07C),
        .method_options = color_hex(0xA89984),

        .status_2xx = color_hex(0xB8BB26),
        .status_3xx = color_hex(0xFABD2F),
        .status_4xx = color_hex(0xFB4934),
        .status_5xx = color_hex(0xFB4934),

        .syntax_key = color_hex(0x83A598),
        .syntax_string = color_hex(0xB8BB26),
        .syntax_number = color_hex(0xD3869B),
        .syntax_boolean = color_hex(0xD3869B),
        .syntax_null = color_hex(0xFABD2F),
        .syntax_punctuation = color_hex(0xA89984),
        .syntax_comment = color_hex(0x928374),
    };
}

Theme* theme_get_active(void) {
    return &s_themes[s_active_preset];
}

void theme_set_preset(ThemePreset preset) {
    if (preset < THEME_COUNT) {
        s_active_preset = preset;
    }
}

const char* theme_get_preset_name(ThemePreset preset) {
    if (preset < THEME_COUNT) {
        return s_themes[preset].name;
    }
    return "Unknown";
}

Color theme_get_method_color(const char* method) {
    Theme* t = theme_get_active();
    if (!method) return t->foreground;
    if (strcasecmp(method, "GET") == 0) return t->method_get;
    if (strcasecmp(method, "POST") == 0) return t->method_post;
    if (strcasecmp(method, "PUT") == 0) return t->method_put;
    if (strcasecmp(method, "PATCH") == 0) return t->method_patch;
    if (strcasecmp(method, "DELETE") == 0) return t->method_delete;
    if (strcasecmp(method, "HEAD") == 0) return t->method_head;
    if (strcasecmp(method, "OPTIONS") == 0) return t->method_options;
    return t->foreground;
}

Color theme_get_status_color(int status_code) {
    Theme* t = theme_get_active();
    if (status_code >= 200 && status_code < 300) return t->status_2xx;
    if (status_code >= 300 && status_code < 400) return t->status_3xx;
    if (status_code >= 400 && status_code < 500) return t->status_4xx;
    if (status_code >= 500) return t->status_5xx;
    return t->text_muted;
}

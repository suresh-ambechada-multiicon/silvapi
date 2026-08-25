#include "app/config.h"
#include "cJSON.h"
#include <unistd.h>

AppConfig g_config;

AppConfig app_config_default(void) {
    AppConfig c = {0};

    c.font_size_ui   = 15.0f;
    c.font_size_mono = 14.0f;
    c.font_size_bold = 15.0f;
    c.font_family_ui[0]   = '\0';
    c.font_family_mono[0] = '\0';

    c.dpi_scale = 0.0f;   // follow the window content scale

    c.gfx  = gfx_config_default();
    c.text = text_config_default();
    c.icon = icon_config_default();

    c.topbar_height    = 42.0f;
    c.row_height       = 28.0f;
    c.corner_radius    = 4.0f;
    c.pad_small        = 4.0f;
    c.pad_medium       = 8.0f;
    c.pad_large        = 12.0f;
    c.gap_small        = 4.0f;
    c.gap_medium       = 6.0f;
    c.icon_size_small  = 14.0f;
    c.icon_size_medium = 16.0f;

    snprintf(c.theme_name, sizeof(c.theme_name), "%s", "ayu-dark");
    return c;
}

// --- JSON helpers ----------------------------------------------------------

static void get_num(const cJSON* obj, const char* key, float* out) {
    if (!obj || !out) return;
    const cJSON* v = cJSON_GetObjectItemCaseSensitive(obj, key);
    if (cJSON_IsNumber(v)) *out = (float)v->valuedouble;
}

static void get_int(const cJSON* obj, const char* key, int* out) {
    if (!obj || !out) return;
    const cJSON* v = cJSON_GetObjectItemCaseSensitive(obj, key);
    if (cJSON_IsNumber(v)) *out = v->valueint;
}

static void get_bool(const cJSON* obj, const char* key, bool* out) {
    if (!obj || !out) return;
    const cJSON* v = cJSON_GetObjectItemCaseSensitive(obj, key);
    if (cJSON_IsBool(v)) *out = cJSON_IsTrue(v);
}

static void get_str(const cJSON* obj, const char* key, char* out, size_t cap) {
    if (!obj || !out || cap == 0) return;
    const cJSON* v = cJSON_GetObjectItemCaseSensitive(obj, key);
    if (cJSON_IsString(v) && v->valuestring) snprintf(out, cap, "%s", v->valuestring);
}

static void get_hinting(const cJSON* obj, const char* key, TextHinting* out) {
    if (!obj || !out) return;
    const cJSON* v = cJSON_GetObjectItemCaseSensitive(obj, key);
    if (!cJSON_IsString(v) || !v->valuestring) return;
    if      (strcasecmp(v->valuestring, "none")   == 0) *out = TEXT_HINT_NONE;
    else if (strcasecmp(v->valuestring, "light")  == 0) *out = TEXT_HINT_LIGHT;
    else if (strcasecmp(v->valuestring, "normal") == 0) *out = TEXT_HINT_NORMAL;
    else fprintf(stderr, "[Config] unknown hinting '%s', keeping default\n", v->valuestring);
}

static char* read_file(const char* path, size_t* out_len) {
    FILE* f = fopen(path, "rb");
    if (!f) return NULL;
    if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return NULL; }
    long n = ftell(f);
    if (n < 0 || fseek(f, 0, SEEK_SET) != 0) { fclose(f); return NULL; }

    char* buf = malloc((size_t)n + 1);
    if (!buf) { fclose(f); return NULL; }
    size_t got = fread(buf, 1, (size_t)n, f);
    fclose(f);
    buf[got] = '\0';
    if (out_len) *out_len = got;
    return buf;
}

static void merge_json(AppConfig* c, const cJSON* root) {
    const cJSON* font = cJSON_GetObjectItemCaseSensitive(root, "font");
    if (font) {
        get_num(font, "size_ui",   &c->font_size_ui);
        get_num(font, "size_mono", &c->font_size_mono);
        get_num(font, "size_bold", &c->font_size_bold);
        get_str(font, "family_ui",   c->font_family_ui,   sizeof(c->font_family_ui));
        get_str(font, "family_mono", c->font_family_mono, sizeof(c->font_family_mono));
    }

    const cJSON* render = cJSON_GetObjectItemCaseSensitive(root, "render");
    if (render) {
        get_num(render,  "dpi_scale",    &c->dpi_scale);
        get_num(render,  "alpha_gamma",  &c->gfx.alpha_gamma);
        get_num(render,  "stem_darken",  &c->gfx.stem_darken);
        get_bool(render, "multisample",  &c->gfx.multisample);
    }

    const cJSON* text = cJSON_GetObjectItemCaseSensitive(root, "text");
    if (text) {
        get_hinting(text, "hinting", &c->text.hinting);
        get_int(text, "subpixel_buckets", &c->text.subpixel_buckets);
    }

    const cJSON* icon = cJSON_GetObjectItemCaseSensitive(root, "icon");
    if (icon) {
        get_num(icon, "stroke_width", &c->icon.stroke_width);
        get_num(icon, "edge_padding", &c->icon.edge_padding);
    }

    const cJSON* layout = cJSON_GetObjectItemCaseSensitive(root, "layout");
    if (layout) {
        get_num(layout, "topbar_height",    &c->topbar_height);
        get_num(layout, "row_height",       &c->row_height);
        get_num(layout, "corner_radius",    &c->corner_radius);
        get_num(layout, "pad_small",        &c->pad_small);
        get_num(layout, "pad_medium",       &c->pad_medium);
        get_num(layout, "pad_large",        &c->pad_large);
        get_num(layout, "gap_small",        &c->gap_small);
        get_num(layout, "gap_medium",       &c->gap_medium);
        get_num(layout, "icon_size_small",  &c->icon_size_small);
        get_num(layout, "icon_size_medium", &c->icon_size_medium);
    }

    get_str(root, "theme", c->theme_name, sizeof(c->theme_name));
}

// --- search path -----------------------------------------------------------

static bool try_path(AppConfig* c, const char* path) {
    if (!path || !*path) return false;

    size_t len = 0;
    char* txt = read_file(path, &len);
    if (!txt) return false;

    cJSON* root = cJSON_Parse(txt);
    if (!root) {
        const char* err = cJSON_GetErrorPtr();
        fprintf(stderr, "[Config] %s: parse error near '%.24s' -- using defaults\n",
                path, err ? err : "?");
        free(txt);
        return false;
    }

    merge_json(c, root);
    snprintf(c->source_path, sizeof(c->source_path), "%s", path);
    cJSON_Delete(root);
    free(txt);
    return true;
}

bool app_config_load(AppConfig* out) {
    if (!out) return false;
    *out = app_config_default();

    const char* env = getenv("SILVAPI_CONFIG");
    if (env && try_path(out, env)) return true;

    char path[512];
    const char* xdg = getenv("XDG_CONFIG_HOME");
    if (xdg && *xdg) {
        snprintf(path, sizeof(path), "%s/silvapi/config.json", xdg);
        if (try_path(out, path)) return true;
    } else {
        const char* home = getenv("HOME");
        if (home && *home) {
            snprintf(path, sizeof(path), "%s/.config/silvapi/config.json", home);
            if (try_path(out, path)) return true;
        }
    }

    if (try_path(out, "silvapi.json")) return true;
    return false;
}

// --- apply -----------------------------------------------------------------

void app_config_apply(const AppConfig* cfg) {
    if (!cfg) return;

    float dpi = cfg->dpi_scale > 0.0f ? cfg->dpi_scale : cfg->text.dpi_scale;
    if (dpi <= 0.0f) dpi = 1.0f;

    TextConfig tc = cfg->text;
    tc.dpi_scale = dpi;
    text_set_config(tc);

    IconConfig ic = cfg->icon;
    ic.dpi_scale = dpi;
    icon_set_config(ic);

    gfx_set_config(cfg->gfx);
}

// --- write -----------------------------------------------------------------

static const char* hinting_name(TextHinting h) {
    switch (h) {
        case TEXT_HINT_NONE:  return "none";
        case TEXT_HINT_LIGHT: return "light";
        default:              return "normal";
    }
}

bool app_config_write(const AppConfig* c, const char* path) {
    if (!c || !path) return false;

    cJSON* root = cJSON_CreateObject();
    if (!root) return false;

    cJSON* font = cJSON_AddObjectToObject(root, "font");
    cJSON_AddNumberToObject(font, "size_ui",   c->font_size_ui);
    cJSON_AddNumberToObject(font, "size_mono", c->font_size_mono);
    cJSON_AddNumberToObject(font, "size_bold", c->font_size_bold);
    cJSON_AddStringToObject(font, "family_ui",   c->font_family_ui);
    cJSON_AddStringToObject(font, "family_mono", c->font_family_mono);

    cJSON* render = cJSON_AddObjectToObject(root, "render");
    cJSON_AddNumberToObject(render, "dpi_scale",   c->dpi_scale);
    cJSON_AddNumberToObject(render, "alpha_gamma", c->gfx.alpha_gamma);
    cJSON_AddNumberToObject(render, "stem_darken", c->gfx.stem_darken);
    cJSON_AddBoolToObject(render,   "multisample", c->gfx.multisample);

    cJSON* text = cJSON_AddObjectToObject(root, "text");
    cJSON_AddStringToObject(text, "hinting", hinting_name(c->text.hinting));
    cJSON_AddNumberToObject(text, "subpixel_buckets", c->text.subpixel_buckets);

    cJSON* icon = cJSON_AddObjectToObject(root, "icon");
    cJSON_AddNumberToObject(icon, "stroke_width", c->icon.stroke_width);
    cJSON_AddNumberToObject(icon, "edge_padding", c->icon.edge_padding);

    cJSON* layout = cJSON_AddObjectToObject(root, "layout");
    cJSON_AddNumberToObject(layout, "topbar_height",    c->topbar_height);
    cJSON_AddNumberToObject(layout, "row_height",       c->row_height);
    cJSON_AddNumberToObject(layout, "corner_radius",    c->corner_radius);
    cJSON_AddNumberToObject(layout, "pad_small",        c->pad_small);
    cJSON_AddNumberToObject(layout, "pad_medium",       c->pad_medium);
    cJSON_AddNumberToObject(layout, "pad_large",        c->pad_large);
    cJSON_AddNumberToObject(layout, "gap_small",        c->gap_small);
    cJSON_AddNumberToObject(layout, "gap_medium",       c->gap_medium);
    cJSON_AddNumberToObject(layout, "icon_size_small",  c->icon_size_small);
    cJSON_AddNumberToObject(layout, "icon_size_medium", c->icon_size_medium);

    cJSON_AddStringToObject(root, "theme", c->theme_name);

    char* txt = cJSON_Print(root);
    cJSON_Delete(root);
    if (!txt) return false;

    FILE* f = fopen(path, "wb");
    if (!f) { free(txt); return false; }
    fputs(txt, f);
    fputc('\n', f);
    fclose(f);
    free(txt);
    return true;
}

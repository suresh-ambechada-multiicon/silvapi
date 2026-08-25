#include "ui/style.h"
#include <ctype.h>

// ---------------------------------------------------------------------------
// Utility-class parser.
//
// Parsing happens once per distinct class string and the result is cached, so a
// call site inside a loop pays a hash lookup. The cache is keyed on the string
// bytes plus a generation counter that bumps whenever tokens or the spacing
// unit change -- colours resolve through the theme, so a theme switch has to
// invalidate every previously parsed style.
// ---------------------------------------------------------------------------

#define STYLE_CACHE_SLOTS   512
#define TOKEN_SLOTS         256
#define TOKEN_NAME_MAX      48
#define UNKNOWN_LOG_SLOTS   64

typedef struct {
    char  name[TOKEN_NAME_MAX];
    Color value;
    bool  used;
} TokenSlot;

typedef struct {
    uint64_t hash;       // 0 = empty
    uint32_t generation;
    UiStyle  style;
} StyleSlot;

static TokenSlot s_tokens[TOKEN_SLOTS];
static StyleSlot s_cache[STYLE_CACHE_SLOTS];
static uint32_t  s_generation = 1;
static float     s_unit = 4.0f;
static float     s_radius_sm = 2.0f, s_radius_md = 4.0f, s_radius_lg = 8.0f;
static bool      s_warn_unknown = false;
static UiStyleStats s_stats;

// Unknown class names are logged once each; a typo is otherwise invisible.
static char   s_unknown_log[UNKNOWN_LOG_SLOTS][TOKEN_NAME_MAX];
static size_t s_unknown_count;

// --- hashing ---------------------------------------------------------------

static uint64_t hash_str(const char* s, uint64_t seed) {
    uint64_t h = seed ? seed : 1469598103934665603ULL;
    for (; s && *s; s++) {
        h ^= (uint8_t)*s;
        h *= 1099511628211ULL;
    }
    return h;
}

// --- tokens ----------------------------------------------------------------

void ui_style_set_token(const char* name, Color value) {
    if (!name || !*name) return;
    uint64_t h = hash_str(name, 0);

    for (int probe = 0; probe < 32; probe++) {
        TokenSlot* t = &s_tokens[(h + (uint64_t)probe) % TOKEN_SLOTS];
        if (t->used && strcmp(t->name, name) == 0) {
            t->value = value;
            return;
        }
        if (!t->used) {
            snprintf(t->name, sizeof(t->name), "%s", name);
            t->value = value;
            t->used = true;
            return;
        }
    }
    fprintf(stderr, "[Style] token table full, '%s' dropped\n", name);
}

bool ui_style_get_token(const char* name, Color* out) {
    if (!name || !*name) return false;
    uint64_t h = hash_str(name, 0);
    for (int probe = 0; probe < 32; probe++) {
        const TokenSlot* t = &s_tokens[(h + (uint64_t)probe) % TOKEN_SLOTS];
        if (!t->used) return false;
        if (strcmp(t->name, name) == 0) {
            if (out) *out = t->value;
            return true;
        }
    }
    return false;
}

void ui_style_clear_tokens(void) {
    memset(s_tokens, 0, sizeof(s_tokens));
    ui_style_invalidate();
}

void ui_style_set_spacing_unit(float px) {
    if (px > 0.0f && px != s_unit) {
        s_unit = px;
        ui_style_invalidate();
    }
}

float ui_style_spacing_unit(void) { return s_unit; }

void ui_style_set_radius_scale(float sm, float md, float lg) {
    s_radius_sm = sm;
    s_radius_md = md;
    s_radius_lg = lg;
    ui_style_invalidate();
}

void ui_style_invalidate(void) { s_generation++; }

void ui_style_set_warn_unknown(bool enabled) { s_warn_unknown = enabled; }

UiStyleStats ui_style_stats(void) { return s_stats; }

static void warn_unknown(const char* tok, size_t len) {
    if (!s_warn_unknown) return;
    char buf[TOKEN_NAME_MAX];
    size_t n = len < sizeof(buf) - 1 ? len : sizeof(buf) - 1;
    memcpy(buf, tok, n);
    buf[n] = '\0';

    for (size_t i = 0; i < s_unknown_count; i++) {
        if (strcmp(s_unknown_log[i], buf) == 0) return;
    }
    if (s_unknown_count < UNKNOWN_LOG_SLOTS) {
        snprintf(s_unknown_log[s_unknown_count++], TOKEN_NAME_MAX, "%s", buf);
    }
    s_stats.unknown_classes++;
    fprintf(stderr, "[Style] unknown class '%s'\n", buf);
}

// --- token scanning helpers ------------------------------------------------

// A class token is a slice of the input; nothing is copied while matching.
typedef struct {
    const char* p;
    size_t      n;
} Slice;

static bool slice_eq(Slice s, const char* lit) {
    size_t n = strlen(lit);
    return s.n == n && memcmp(s.p, lit, n) == 0;
}

static bool slice_prefix(Slice s, const char* lit, Slice* rest) {
    size_t n = strlen(lit);
    if (s.n < n || memcmp(s.p, lit, n) != 0) return false;
    if (rest) {
        rest->p = s.p + n;
        rest->n = s.n - n;
    }
    return true;
}

// Parses a numeric suffix. "3" -> 3 * unit; "28px" -> 28; "1/2" -> fraction.
// Returns false when the slice is not a number.
static bool parse_amount_scaled(Slice s, float* out_px, float* out_fraction, bool scale) {
    if (s.n == 0) return false;
    if (out_fraction) *out_fraction = -1.0f;

    // Fraction form n/m
    for (size_t i = 0; i < s.n; i++) {
        if (s.p[i] != '/') continue;
        char a[16], b[16];
        size_t an = i, bn = s.n - i - 1;
        if (an == 0 || an >= sizeof(a) || bn == 0 || bn >= sizeof(b)) return false;
        memcpy(a, s.p, an); a[an] = '\0';
        memcpy(b, s.p + i + 1, bn); b[bn] = '\0';
        float num = strtof(a, NULL), den = strtof(b, NULL);
        if (den == 0.0f) return false;
        if (out_fraction) *out_fraction = num / den;
        return true;
    }

    bool raw_px = (s.n > 2 && memcmp(s.p + s.n - 2, "px", 2) == 0);
    size_t digits = raw_px ? s.n - 2 : s.n;

    char buf[24];
    if (digits == 0 || digits >= sizeof(buf)) return false;
    for (size_t i = 0; i < digits; i++) {
        if (!isdigit((unsigned char)s.p[i]) && s.p[i] != '.' && s.p[i] != '-') return false;
    }
    memcpy(buf, s.p, digits);
    buf[digits] = '\0';

    float v = strtof(buf, NULL);
    if (out_px) *out_px = (raw_px || !scale) ? v : v * s_unit;
    return true;
}

static bool parse_amount(Slice s, float* out_px, float* out_fraction) {
    return parse_amount_scaled(s, out_px, out_fraction, true);
}

// For values that are counts rather than distances: border widths, z indexes.
static bool parse_raw(Slice s, float* out) {
    return parse_amount_scaled(s, out, NULL, false);
}

// Resolves a colour token slice, e.g. the "primary" in "bg-primary".
static bool resolve_color(Slice s, Color* out) {
    char name[TOKEN_NAME_MAX];
    if (s.n == 0 || s.n >= sizeof(name)) return false;
    memcpy(name, s.p, s.n);
    name[s.n] = '\0';

    if (ui_style_get_token(name, out)) return true;

    // Literal hex: bg-#1a1a1a or bg-#1a1a1aff
    if (name[0] == '#') {
        size_t hexlen = strlen(name + 1);
        if (hexlen == 6 || hexlen == 8) {
            uint32_t v = (uint32_t)strtoul(name + 1, NULL, 16);
            *out = (hexlen == 6) ? color_hex(v)
                                 : color_rgba(((v >> 24) & 0xFF) / 255.0f,
                                              ((v >> 16) & 0xFF) / 255.0f,
                                              ((v >> 8) & 0xFF) / 255.0f,
                                              (v & 0xFF) / 255.0f);
            return true;
        }
    }
    if (slice_eq(s, "transparent")) {
        *out = (Color){0, 0, 0, 0};
        return true;
    }
    return false;
}

// --- single class application ---------------------------------------------

static void set_pad(UiStyle* st, float l, float r, float t, float b) {
    st->pad_l = l; st->pad_r = r; st->pad_t = t; st->pad_b = b;
    st->set_mask |= UI_STYLE_SET_PAD;
}

static void apply_class(UiStyle* st, Slice tok) {
    // State prefix.
    UiStateKind state = UI_STATE_BASE;
    Slice rest;
    if      (slice_prefix(tok, "hover:", &rest))    { state = UI_STATE_HOVER;    tok = rest; }
    else if (slice_prefix(tok, "active:", &rest))   { state = UI_STATE_ACTIVE;   tok = rest; }
    else if (slice_prefix(tok, "focus:", &rest))    { state = UI_STATE_FOCUS;    tok = rest; }
    else if (slice_prefix(tok, "selected:", &rest)) { state = UI_STATE_SELECTED; tok = rest; }
    else if (slice_prefix(tok, "disabled:", &rest)) { state = UI_STATE_DISABLED; tok = rest; }

    float px = 0.0f, frac = -1.0f;

    // --- direction ---
    if (slice_eq(tok, "row")) { st->direction = 0; st->set_mask |= UI_STYLE_SET_DIR; return; }
    if (slice_eq(tok, "col")) { st->direction = 1; st->set_mask |= UI_STYLE_SET_DIR; return; }

    // --- size ---
    if (slice_eq(tok, "w-full")) { st->w = UI_GROW; st->set_mask |= UI_STYLE_SET_W; return; }
    if (slice_eq(tok, "h-full")) { st->h = UI_GROW; st->set_mask |= UI_STYLE_SET_H; return; }
    if (slice_eq(tok, "w-fit"))  { st->w = UI_FIT;  st->set_mask |= UI_STYLE_SET_W; return; }
    if (slice_eq(tok, "h-fit"))  { st->h = UI_FIT;  st->set_mask |= UI_STYLE_SET_H; return; }

    if (slice_prefix(tok, "w-min-", &rest) && parse_amount(rest, &px, NULL)) {
        st->w = UI_GROW_MIN(px); st->set_mask |= UI_STYLE_SET_W; return;
    }
    if (slice_prefix(tok, "h-min-", &rest) && parse_amount(rest, &px, NULL)) {
        st->h = UI_GROW_MIN(px); st->set_mask |= UI_STYLE_SET_H; return;
    }
    if (slice_prefix(tok, "w-", &rest) && parse_amount(rest, &px, &frac)) {
        st->w = (frac >= 0.0f) ? UI_PCT(frac) : UI_PX(px);
        st->set_mask |= UI_STYLE_SET_W;
        return;
    }
    if (slice_prefix(tok, "h-", &rest) && parse_amount(rest, &px, &frac)) {
        st->h = (frac >= 0.0f) ? UI_PCT(frac) : UI_PX(px);
        st->set_mask |= UI_STYLE_SET_H;
        return;
    }

    // --- spacing ---
    if (slice_prefix(tok, "px-", &rest) && parse_amount(rest, &px, NULL)) {
        set_pad(st, px, px, st->pad_t, st->pad_b); return;
    }
    if (slice_prefix(tok, "py-", &rest) && parse_amount(rest, &px, NULL)) {
        set_pad(st, st->pad_l, st->pad_r, px, px); return;
    }
    if (slice_prefix(tok, "pl-", &rest) && parse_amount(rest, &px, NULL)) {
        set_pad(st, px, st->pad_r, st->pad_t, st->pad_b); return;
    }
    if (slice_prefix(tok, "pr-", &rest) && parse_amount(rest, &px, NULL)) {
        set_pad(st, st->pad_l, px, st->pad_t, st->pad_b); return;
    }
    if (slice_prefix(tok, "pt-", &rest) && parse_amount(rest, &px, NULL)) {
        set_pad(st, st->pad_l, st->pad_r, px, st->pad_b); return;
    }
    if (slice_prefix(tok, "pb-", &rest) && parse_amount(rest, &px, NULL)) {
        set_pad(st, st->pad_l, st->pad_r, st->pad_t, px); return;
    }
    if (slice_prefix(tok, "p-", &rest) && parse_amount(rest, &px, NULL)) {
        set_pad(st, px, px, px, px); return;
    }
    if (slice_prefix(tok, "gap-", &rest) && parse_amount(rest, &px, NULL)) {
        st->gap = px; st->set_mask |= UI_STYLE_SET_GAP; return;
    }

    // --- alignment ---
    if (slice_eq(tok, "items-start"))  { st->align_y = UI_START;  st->set_mask |= UI_STYLE_SET_ALIGN_Y; return; }
    if (slice_eq(tok, "items-center")) { st->align_y = UI_CENTER; st->set_mask |= UI_STYLE_SET_ALIGN_Y; return; }
    if (slice_eq(tok, "items-end"))    { st->align_y = UI_END;    st->set_mask |= UI_STYLE_SET_ALIGN_Y; return; }
    if (slice_eq(tok, "justify-start"))  { st->align_x = UI_START;  st->set_mask |= UI_STYLE_SET_ALIGN_X; return; }
    if (slice_eq(tok, "justify-center")) { st->align_x = UI_CENTER; st->set_mask |= UI_STYLE_SET_ALIGN_X; return; }
    if (slice_eq(tok, "justify-end"))    { st->align_x = UI_END;    st->set_mask |= UI_STYLE_SET_ALIGN_X; return; }

    // --- radius ---
    if (slice_eq(tok, "rounded-none")) { st->radius = 0.0f;        st->set_mask |= UI_STYLE_SET_RADIUS; return; }
    if (slice_eq(tok, "rounded-sm"))   { st->radius = s_radius_sm; st->set_mask |= UI_STYLE_SET_RADIUS; return; }
    if (slice_eq(tok, "rounded-md"))   { st->radius = s_radius_md; st->set_mask |= UI_STYLE_SET_RADIUS; return; }
    if (slice_eq(tok, "rounded-lg"))   { st->radius = s_radius_lg; st->set_mask |= UI_STYLE_SET_RADIUS; return; }
    if (slice_eq(tok, "rounded-full")) { st->radius = 9999.0f;     st->set_mask |= UI_STYLE_SET_RADIUS; return; }
    if (slice_eq(tok, "rounded"))      { st->radius = s_radius_md; st->set_mask |= UI_STYLE_SET_RADIUS; return; }

    // --- colour ---
    Color c;
    if (slice_prefix(tok, "bg-", &rest) && resolve_color(rest, &c)) {
        st->bg[state] = c;
        st->has_bg |= (uint8_t)(1u << state);
        return;
    }
    if (slice_prefix(tok, "text-", &rest) && resolve_color(rest, &c)) {
        st->fg[state] = c;
        st->has_fg |= (uint8_t)(1u << state);
        return;
    }
    if (slice_prefix(tok, "border-", &rest) && resolve_color(rest, &c)) {
        st->border_color[state] = c;
        st->has_border_color |= (uint8_t)(1u << state);
        if (!(st->set_mask & UI_STYLE_SET_BORDER)) {
            st->border = 1.0f;
            st->set_mask |= UI_STYLE_SET_BORDER;
        }
        return;
    }

    // --- border width ---
    if (slice_eq(tok, "border")) { st->border = 1.0f; st->set_mask |= UI_STYLE_SET_BORDER; return; }
    if (slice_prefix(tok, "border-", &rest) && parse_raw(rest, &px)) {
        st->border = px; st->set_mask |= UI_STYLE_SET_BORDER; return;
    }

    // --- text ---
    if (slice_eq(tok, "font-normal")) { st->font = UI_FONT_UI;   st->set_mask |= UI_STYLE_SET_FONT; return; }
    if (slice_eq(tok, "font-bold"))   { st->font = UI_FONT_BOLD; st->set_mask |= UI_STYLE_SET_FONT; return; }
    if (slice_eq(tok, "font-mono"))   { st->font = UI_FONT_MONO; st->set_mask |= UI_STYLE_SET_FONT; return; }
    if (slice_eq(tok, "text-left"))   { st->text_align = TEXT_ALIGN_LEFT;   st->set_mask |= UI_STYLE_SET_TEXT_AL; return; }
    if (slice_eq(tok, "text-center")) { st->text_align = TEXT_ALIGN_CENTER; st->set_mask |= UI_STYLE_SET_TEXT_AL; return; }
    if (slice_eq(tok, "text-right"))  { st->text_align = TEXT_ALIGN_RIGHT;  st->set_mask |= UI_STYLE_SET_TEXT_AL; return; }
    if (slice_eq(tok, "truncate"))    { st->truncate = true; st->set_mask |= UI_STYLE_SET_TRUNCATE; return; }

    // --- overflow ---
    if (slice_eq(tok, "clip"))     { st->clip_x = st->clip_y = true; st->set_mask |= UI_STYLE_SET_OVERFLOW; return; }
    if (slice_eq(tok, "clip-x"))   { st->clip_x = true;   st->set_mask |= UI_STYLE_SET_OVERFLOW; return; }
    if (slice_eq(tok, "clip-y"))   { st->clip_y = true;   st->set_mask |= UI_STYLE_SET_OVERFLOW; return; }
    if (slice_eq(tok, "scroll-x")) { st->scroll_x = true; st->set_mask |= UI_STYLE_SET_OVERFLOW; return; }
    if (slice_eq(tok, "scroll-y")) { st->scroll_y = true; st->set_mask |= UI_STYLE_SET_OVERFLOW; return; }

    // --- floating ---
    if (slice_eq(tok, "float"))      { st->floating = true; st->set_mask |= UI_STYLE_SET_FLOAT; return; }
    if (slice_eq(tok, "float-root")) { st->floating = true; st->float_to_root = true;
                                       st->set_mask |= UI_STYLE_SET_FLOAT; return; }
    if (slice_eq(tok, "float-clip"))  { st->float_clip_to_parent = true;
                                       st->set_mask |= UI_STYLE_SET_FLOAT; return; }
    if (slice_prefix(tok, "z-", &rest) && parse_raw(rest, &px)) {
        st->float_z = (int)px;
        st->set_mask |= UI_STYLE_SET_FLOAT;
        return;
    }
    if (slice_prefix(tok, "at-", &rest)) {
        static const struct { const char* n; UiAttach a; } anchors[] = {
            {"top-left", UI_ATTACH_TOP_LEFT},        {"top-center", UI_ATTACH_TOP_CENTER},
            {"top-right", UI_ATTACH_TOP_RIGHT},      {"center-left", UI_ATTACH_CENTER_LEFT},
            {"center", UI_ATTACH_CENTER},            {"center-right", UI_ATTACH_CENTER_RIGHT},
            {"bottom-left", UI_ATTACH_BOTTOM_LEFT},  {"bottom-center", UI_ATTACH_BOTTOM_CENTER},
            {"bottom-right", UI_ATTACH_BOTTOM_RIGHT},
        };
        for (size_t i = 0; i < sizeof(anchors) / sizeof(anchors[0]); i++) {
            if (slice_eq(rest, anchors[i].n)) {
                st->float_attach = anchors[i].a;
                st->set_mask |= UI_STYLE_SET_FLOAT;
                return;
            }
        }
    }

    warn_unknown(tok.p, tok.n);
}

static void parse_into(UiStyle* st, const char* classes) {
    if (!classes) return;
    const char* p = classes;
    while (*p) {
        while (*p == ' ' || *p == '\t' || *p == '\n') p++;
        if (!*p) break;
        const char* start = p;
        while (*p && *p != ' ' && *p != '\t' && *p != '\n') p++;
        apply_class(st, (Slice){ start, (size_t)(p - start) });
    }
}

// --- cache -----------------------------------------------------------------

static const UiStyle* cache_lookup_or_parse(const char* a, const char* b) {
    uint64_t h = hash_str(a, 0);
    if (b) h = hash_str(b, h ^ 0x9E3779B97F4A7C15ULL);
    if (h == 0) h = 1;

    StyleSlot* victim = NULL;
    for (int probe = 0; probe < 8; probe++) {
        StyleSlot* slot = &s_cache[(h + (uint64_t)probe) % STYLE_CACHE_SLOTS];
        if (slot->hash == h && slot->generation == s_generation) {
            s_stats.hits++;
            return &slot->style;
        }
        // A stale-generation entry is free to reuse.
        if (!victim && (slot->hash == 0 || slot->generation != s_generation)) victim = slot;
    }
    if (!victim) victim = &s_cache[h % STYLE_CACHE_SLOTS];

    UiStyle st = {0};
    parse_into(&st, a);
    if (b) parse_into(&st, b);   // overrides applied after, so they win

    if (victim->hash == 0) s_stats.entries++;
    victim->hash = h;
    victim->generation = s_generation;
    victim->style = st;
    s_stats.parses++;
    return &victim->style;
}

const UiStyle* ui_style(const char* classes) {
    static const UiStyle empty = {0};
    if (!classes || !*classes) return &empty;
    return cache_lookup_or_parse(classes, NULL);
}

const UiStyle* ui_style_merge(const char* base, const char* overrides) {
    if (!overrides || !*overrides) return ui_style(base);
    if (!base || !*base)          return ui_style(overrides);
    return cache_lookup_or_parse(base, overrides);
}

// --- application -----------------------------------------------------------

// Falls back to the base state when a state has no value of its own, which is
// what makes "bg-surface hover:bg-accent" behave the way it reads.
static Color pick(const Color* arr, uint8_t mask, UiStateKind state) {
    if (mask & (1u << state)) return arr[state];
    if (mask & (1u << UI_STATE_BASE)) return arr[UI_STATE_BASE];
    return (Color){0, 0, 0, 0};
}

Color ui_style_bg(const UiStyle* s, UiStateKind state) {
    return s ? pick(s->bg, s->has_bg, state) : (Color){0, 0, 0, 0};
}

Color ui_style_fg(const UiStyle* s, UiStateKind state) {
    return s ? pick(s->fg, s->has_fg, state) : (Color){0, 0, 0, 0};
}

UiBoxDesc ui_style_box(const UiStyle* s, const char* id, int32_t index, UiStateKind state) {
    UiBoxDesc d = {0};
    if (!s) return d;

    d.id = id;
    d.index = index;
    d.w = s->w;
    d.h = s->h;
    d.pad_l = s->pad_l;
    d.pad_r = s->pad_r;
    d.pad_t = s->pad_t;
    d.pad_b = s->pad_b;
    d.gap = s->gap;
    d.align_x = s->align_x;
    d.align_y = s->align_y;
    d.radius = s->radius;
    d.bg = pick(s->bg, s->has_bg, state);
    d.border = s->border;
    d.border_color = pick(s->border_color, s->has_border_color, state);
    d.clip_x = s->clip_x;
    d.clip_y = s->clip_y;
    d.scroll_x = s->scroll_x;
    d.scroll_y = s->scroll_y;
    d.floating = s->floating;
    d.float_to_root = s->float_to_root;
    d.float_clip_to_parent = s->float_clip_to_parent;
    d.float_attach = s->float_attach;
    d.float_z = s->float_z;
    return d;
}

UiTextDesc ui_style_text(const UiStyle* s, UiStateKind state) {
    UiTextDesc t = {0};
    if (!s) return t;
    t.color = pick(s->fg, s->has_fg, state);
    t.font = s->font;
    t.align = s->text_align;
    t.ellipsize = s->truncate;
    return t;
}

// --- variants --------------------------------------------------------------

const char* ui_variant(const UiVariant* table, size_t count, const char* name) {
    if (!table || !name) return "";
    for (size_t i = 0; i < count; i++) {
        if (table[i].name && strcmp(table[i].name, name) == 0) {
            return table[i].classes ? table[i].classes : "";
        }
    }
    return "";
}

#ifndef UI_STYLE_H
#define UI_STYLE_H

#include "base/base.h"
#include "ui/layout.h"

// ---------------------------------------------------------------------------
// Utility-class styling.
//
// Components take a style string instead of two dozen colour and metric fields:
//
//     ui_button(&(UiButtonProps){
//         .label = "Send",
//         .class = "h-8 px-3 gap-2 rounded-md bg-primary text-primary-fg "
//                  "hover:bg-primary-hover",
//     });
//
// The string is parsed ONCE and cached by hash, so a call site inside a loop
// costs a hash lookup rather than a parse. The cache is invalidated when the
// theme changes, since colour tokens resolve through the active theme.
//
// This exists because the alternative -- passing explicit colours everywhere --
// is what produced call sites like the old top bar, where every button repeated
// six colour fields and one typo silently changed the look of one control.
//
// Supported classes
// -----------------
//   direction   row col
//   size        w-<n> h-<n> w-full h-full w-fit h-fit w-px h-px w-1/2 w-1/3 w-2/3
//               w-min-<n> h-min-<n>   (minimum size for a growing axis)
//   spacing     p-<n> px-<n> py-<n> pt-<n> pr-<n> pb-<n> pl-<n> gap-<n>
//   align       items-start items-center items-end
//               justify-start justify-center justify-end
//   colour      bg-<token> text-<token> border-<token>
//   border      border border-<n>          (uniform only; per-side is not
//                                           supported -- use a 1px box as a rule)
//   radius      rounded rounded-none rounded-sm rounded-md rounded-lg rounded-full
//   text        font-normal font-bold font-mono text-left text-center text-right
//               truncate
//   overflow    clip clip-x clip-y scroll-y scroll-x
//   float       float float-root float-clip z-<n> at-<anchor>
//
// `<n>` is in spacing units (4px by default, configurable), so `px-3` is 12px.
// Suffix `px` for a raw pixel value: `h-28px`.
//
// State prefixes: hover:, active:, focus:, selected:, disabled:
// e.g. "bg-surface hover:bg-accent selected:bg-primary"
// ---------------------------------------------------------------------------

typedef enum {
    UI_STATE_BASE = 0,
    UI_STATE_HOVER,
    UI_STATE_ACTIVE,
    UI_STATE_FOCUS,
    UI_STATE_SELECTED,
    UI_STATE_DISABLED,
    UI_STATE_COUNT
} UiStateKind;

// Which fields a style string actually set. A component merges its variant
// defaults with a caller's overrides, and needs to know which is which -- a
// zeroed Color is indistinguishable from "transparent" otherwise.
typedef enum {
    UI_STYLE_SET_W        = 1u << 0,
    UI_STYLE_SET_H        = 1u << 1,
    UI_STYLE_SET_PAD      = 1u << 2,
    UI_STYLE_SET_GAP      = 1u << 3,
    UI_STYLE_SET_ALIGN_X  = 1u << 4,
    UI_STYLE_SET_ALIGN_Y  = 1u << 5,
    UI_STYLE_SET_DIR      = 1u << 6,
    UI_STYLE_SET_RADIUS   = 1u << 7,
    UI_STYLE_SET_BORDER   = 1u << 8,
    UI_STYLE_SET_FONT     = 1u << 9,
    UI_STYLE_SET_TEXT_AL  = 1u << 10,
    UI_STYLE_SET_TRUNCATE = 1u << 11,
    UI_STYLE_SET_OVERFLOW = 1u << 12,
    UI_STYLE_SET_FLOAT    = 1u << 13,
} UiStyleSetBits;

typedef struct {
    UiSize   w, h;
    float    pad_l, pad_r, pad_t, pad_b;
    float    gap;
    UiAlign  align_x, align_y;
    int      direction;      // 0 = row, 1 = column
    float    radius;
    float    border;

    Color    bg[UI_STATE_COUNT];
    Color    fg[UI_STATE_COUNT];
    Color    border_color[UI_STATE_COUNT];
    // Per-state presence, so hover:bg can be absent without reading as clear.
    uint8_t  has_bg, has_fg, has_border_color;

    UiFontId  font;
    TextAlign text_align;
    bool      truncate;

    bool     clip_x, clip_y, scroll_x, scroll_y;

    bool     floating, float_to_root, float_clip_to_parent;
    UiAttach float_attach;
    int      float_z;

    uint32_t set_mask;
} UiStyle;

// --- tokens ----------------------------------------------------------------

// Colour tokens resolve by name. The application registers them from its theme
// (see ui_style_load_theme_tokens) so components never name a theme field.
void  ui_style_set_token(const char* name, Color value);
bool  ui_style_get_token(const char* name, Color* out);
void  ui_style_clear_tokens(void);

// Spacing unit for `p-2` style values; default 4.
void  ui_style_set_spacing_unit(float px);
float ui_style_spacing_unit(void);

// Named radius scale used by rounded-sm/md/lg.
void  ui_style_set_radius_scale(float sm, float md, float lg);

// Bumps the cache generation. Call after changing tokens or the spacing unit,
// otherwise previously parsed styles keep their old colours.
void  ui_style_invalidate(void);

// --- parsing ---------------------------------------------------------------

// Parses (or returns cached) style for `classes`. NULL or "" gives a zeroed
// style with set_mask == 0.
const UiStyle* ui_style(const char* classes);

// Parses `base` then applies `overrides` on top, honouring set_mask so an
// override only replaces what it actually specifies. Result is cached on the
// pair, so this is cheap inside a loop.
const UiStyle* ui_style_merge(const char* base, const char* overrides);

// --- application -----------------------------------------------------------

// Builds a UiBoxDesc from a style plus the widget's current interaction state.
UiBoxDesc ui_style_box(const UiStyle* s, const char* id, int32_t index, UiStateKind state);

// Resolves the foreground colour for a state, falling back to the base state.
Color ui_style_fg(const UiStyle* s, UiStateKind state);
Color ui_style_bg(const UiStyle* s, UiStateKind state);

// Text descriptor from a style.
UiTextDesc ui_style_text(const UiStyle* s, UiStateKind state);

// Registers the active theme's colours as tokens ("bg-surface", "text-muted",
// "bg-primary", ...) and invalidates the style cache. Call at startup and after
// any theme change. Defined in style_theme.c, the only file that maps theme
// fields onto token names.
void ui_style_load_theme_tokens(void);

// --- variants (cva-like) ---------------------------------------------------

typedef struct {
    const char* name;
    const char* classes;
} UiVariant;

// Looks up `name` in `table`; returns "" when absent. Used by components to map
// .variant = "ghost" onto a class string.
const char* ui_variant(const UiVariant* table, size_t count, const char* name);

// Diagnostics: how many distinct style strings are cached, and how many parses
// have happened. A parse count that grows every frame means a caller is
// building class strings dynamically and defeating the cache.
typedef struct {
    size_t entries;
    size_t parses;
    size_t hits;
    size_t unknown_classes;
} UiStyleStats;
UiStyleStats ui_style_stats(void);

// Logs every class token the parser did not recognise, once each. Typos in a
// style string are otherwise silent.
void ui_style_set_warn_unknown(bool enabled);

#endif // UI_STYLE_H

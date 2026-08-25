#ifndef UI_LAYOUT_H
#define UI_LAYOUT_H

#include "base/base.h"
#include "text/font.h"
#include "icons/icon.h"

// ---------------------------------------------------------------------------
// Declarative layout.
//
// A thin, typed wrapper over the Clay layout engine. Clay solves the box
// problem well (flex sizing, padding, gaps, clipping, scroll containers) but
// its raw API is macro-heavy and leaks its own vector/colour/string types
// through every call site. This wrapper keeps Clay entirely internal: callers
// only ever see Rect, Color, and const char*.
//
// Text is deliberately NOT left to Clay. Clay breaks lines on spaces only,
// which is wrong for CJK, hyphenated words, and long URLs. Instead the text
// engine measures and wraps, and Clay is handed the resulting block size.
//
// Usage:
//
//     UI_COL(.id = "sidebar", .w = UI_PX(240), .h = UI_GROW, .pad = 8, .gap = 4,
//            .bg = theme->panel) {
//         UI_ROW(.h = UI_PX(28), .gap = 6, .align_y = UI_CENTER) {
//             UI_ICON(ICON_FOLDER, 16, theme->text_muted);
//             UI_LABEL("Collections", .color = theme->foreground);
//         }
//         if (UI_CLICKED()) { ... }
//     }
//
// Blocks nest with plain C braces; there is no begin/end to forget.
// ---------------------------------------------------------------------------

// --- sizing ----------------------------------------------------------------

typedef enum {
    UI_SIZE_FIT = 0,   // shrink to fit children (Clay's default)
    UI_SIZE_GROW,      // fill remaining space along the parent's axis
    UI_SIZE_FIXED,
    UI_SIZE_PERCENT,
} UiSizeKind;

typedef struct {
    UiSizeKind kind;
    float value;
    float min, max;
} UiSize;

#define UI_FIT        ((UiSize){ UI_SIZE_FIT, 0, 0, 0 })
#define UI_GROW       ((UiSize){ UI_SIZE_GROW, 0, 0, 0 })
#define UI_PX(v)      ((UiSize){ UI_SIZE_FIXED, (float)(v), 0, 0 })
#define UI_PCT(v)     ((UiSize){ UI_SIZE_PERCENT, (float)(v), 0, 0 })
#define UI_GROW_MIN(v) ((UiSize){ UI_SIZE_GROW, 0, (float)(v), 0 })

// --- alignment -------------------------------------------------------------

typedef enum { UI_START = 0, UI_CENTER, UI_END } UiAlign;

// Anchor for a floating element. The same point is used on the element and on
// its parent, so UI_ATTACH_CENTER centres it and UI_ATTACH_BOTTOM_CENTER pins
// it to the bottom edge, centred.
typedef enum {
    UI_ATTACH_TOP_LEFT = 0,
    UI_ATTACH_TOP_CENTER,
    UI_ATTACH_TOP_RIGHT,
    UI_ATTACH_CENTER_LEFT,
    UI_ATTACH_CENTER,
    UI_ATTACH_CENTER_RIGHT,
    UI_ATTACH_BOTTOM_LEFT,
    UI_ATTACH_BOTTOM_CENTER,
    UI_ATTACH_BOTTOM_RIGHT,
} UiAttach;

// --- element declaration ---------------------------------------------------

typedef struct {
    // Stable identity. Required for hover/click queries and for scroll state to
    // persist across frames. Elements without an id get a positional id, which
    // is fine for pure decoration.
    const char* id;
    // Disambiguates ids generated inside a loop: pass the loop index.
    int32_t     index;

    UiSize w, h;

    // `pad` sets all four sides; pad_x/pad_y override an axis; pad_l/r/t/b
    // override one side. Later fields win.
    float pad, pad_x, pad_y, pad_l, pad_r, pad_t, pad_b;
    float gap;

    UiAlign align_x, align_y;

    Color bg;
    float radius;
    float border;
    Color border_color;

    // Clips children and enables scrolling on the given axes. Scroll offsets are
    // owned by the layout engine and keyed by element id, so they survive across
    // frames without the caller storing anything.
    bool clip_x, clip_y;
    bool scroll_x, scroll_y;

    // Draws above siblings without affecting their layout (menus, tooltips,
    // modals). This is the ONLY way to make elements overlap: row and column
    // siblings are sized from measured content and cannot collide.
    bool     floating;
    UiAttach float_attach;   // anchor point, on both this element and its parent
    Vec2     float_offset;   // additional offset from the anchor
    int      float_z;
    bool     float_to_root;  // anchor to the layout root instead of the parent
    // Floating elements do NOT inherit their parent's clip by default, so a
    // floating child of a scroll viewport will happily draw outside it. Set this
    // for absolutely-positioned content that must stay inside its container.
    bool     float_clip_to_parent;
} UiBoxDesc;

// --- text ------------------------------------------------------------------

typedef enum { UI_FONT_UI = 0, UI_FONT_MONO = 1, UI_FONT_BOLD = 2 } UiFontId;

typedef struct {
    Color     color;
    UiFontId  font;
    float     line_height;  // 0 = the font's natural line height
    TextAlign align;
    bool      wrap;         // paragraph; see uil_paragraph

    // Truncate with a trailing ellipsis when the text exceeds the space the
    // parent gives it, instead of being hard-clipped mid-glyph. Needs a parent
    // that constrains width (UI_GROW or a fixed size), since the label itself
    // reports its natural width to the solver.
    bool      ellipsize;
} UiTextDesc;

// --- frame lifecycle -------------------------------------------------------

// `fonts` maps UiFontId to a loaded Font*. The array must outlive the layout.
bool uil_layout_init(Font* const* fonts, size_t font_count);
void uil_layout_shutdown(void);

typedef struct {
    Vec2  mouse;
    bool  mouse_down[3];
    bool  mouse_clicked[3];   // pressed this frame (edge)
    Vec2  scroll_delta;
    float dt;
} UiLayoutInput;

// Feeds input and opens the root element. `root` is the region the layout fills.
void uil_layout_begin(Rect root, UiLayoutInput input);

// Closes the root, runs the solver, and emits draw calls through gfx.
void uil_layout_end(void);

// --- overlap validation ----------------------------------------------------

// Siblings laid out in a row or column cannot overlap: their size comes from
// measured content, not from a hardcoded pitch. Overlap is only reachable
// deliberately, via a floating element.
//
// This pass verifies that invariant actually holds at runtime by checking every
// emitted text command against every other at the same z-index. It exists so
// "text never overlaps" is a testable claim rather than an assertion. O(n^2) in
// text runs, so it is opt-in: enabled by SILVAPI_CHECK_OVERLAP=1.
void   uil_set_overlap_check(bool enabled);
bool   uil_overlap_check_enabled(void);

// Overlaps detected during the most recent uil_layout_end().
size_t uil_overlap_count(void);

// Tints every text box so clipping or overlap problems are visible directly.
void   uil_set_debug_text_boxes(bool enabled);

// Logs each button's solved rect plus its hover/click state, which is how an
// unresponsive control gets diagnosed without a human at the mouse.
void   uil_set_trace_buttons(bool enabled);

// --- queries ---------------------------------------------------------------

// Valid inside a UI_ROW/UI_COL/UI_BOX body: did the pointer land on this
// element this frame?
bool uil_hovered(void);

// Bounds of the element with `id` as of the last completed frame. Returns an
// empty rect if unknown. Use for hit-testing outside the declaration pass.
Rect uil_element_rect(const char* id, int32_t index);
bool uil_element_hovered(const char* id, int32_t index);

// Scroll offset owned by the layout for a scrolling element.
Vec2 uil_scroll_offset(const char* id, int32_t index);

// --- leaf elements ---------------------------------------------------------

// Single line, ellipsized if it overflows. Measured through the text engine so
// the reserved width is exactly the drawn width.
void uil_label(const char* text, UiTextDesc desc);

// Wrapped paragraph. The text engine performs the line breaking; the layout is
// handed the resulting block size.
void uil_paragraph(const char* text, UiTextDesc desc);

// A `size` x `size` icon.
void uil_icon(IconType icon, float size, Color color);

// Fixed or growing empty space.
void uil_spacer(UiSize size);

// --- custom drawing --------------------------------------------------------

// Reserves a normal layout box and hands its solved rect to `draw`. This is the
// escape hatch for content the layout engine cannot describe (a code editor, a
// data table, a caret). It still participates in layout, so it cannot overlap
// its siblings -- only what it draws *inside* its own rect is up to the callback.
typedef void (*UiDrawFn)(Rect rect, void* user_data);

void uil_custom(UiSize w, UiSize h, UiDrawFn draw, void* user_data);

// --- floating / modal ------------------------------------------------------

typedef struct {
    const char* id;
    float w, h;              // 0 = fit content
    Color bg, border_color, scrim;
    float radius, border;
    float pad;
    int   z;
} UiModalDesc;

// Opens a centred floating panel over a full-window scrim. Overlapping the
// content beneath is the entire point here, which is why it is floating and not
// a row/column sibling: absolute placement is opt-in.
void uil_modal_begin(UiModalDesc desc);
void uil_modal_end(void);

// UI_MODAL takes field initializers; UI_MODAL_DESC takes a prebuilt struct, so
// shared styling can be factored into a helper.
#define UI_MODAL(...)      UI_MODAL_DESC((UiModalDesc){ __VA_ARGS__ })
#define UI_MODAL_DESC(d) \
    for (int uil__modal_once = (uil_modal_begin(d), 0); \
         uil__modal_once < 1; ++uil__modal_once, uil_modal_end())

// --- stateful widgets ------------------------------------------------------

typedef struct {
    const char* id;
    int32_t     index;
    bool*       value;
    float       size;
    Color       bg, border_color, check_color;
    float       radius;
} UiCheckboxDesc;

// Returns true when toggled this frame.
bool uil_checkbox(UiCheckboxDesc desc);
#define UI_CHECKBOX(...) uil_checkbox((UiCheckboxDesc){ __VA_ARGS__ })

typedef struct {
    const char* id;
    int32_t     index;
    char*       buffer;
    size_t      buffer_size;
    const char* placeholder;
    UiSize      w, h;
    Color       bg, fg, placeholder_color, border_color, border_focus_color;
    Color       caret_color, selection_color;
    float       radius, border, pad_x;
    UiFontId    font;
    bool        readonly;
    bool        password;
} UiTextInputDesc;

// Returns true when the buffer changed this frame. Caret and focus are retained
// per id, so the caller stores nothing.
bool uil_text_input(UiTextInputDesc desc);
#define UI_TEXT_INPUT(...) uil_text_input((UiTextInputDesc){ __VA_ARGS__ })

// The Font behind a UiFontId, for components that must measure before layout
// (reserving a column width, for instance). NULL if the id is unregistered.
Font* uil_font(UiFontId id);

// Currently focused widget id hash, 0 when none. Lets the app route keys.
uint64_t uil_focused(void);
void     uil_clear_focus(void);
// Components with their own focus key (a code editor owns its caret, not a
// text input's) set focus directly.
void     uil_set_focus(uint64_t key);

// --- button ----------------------------------------------------------------

typedef struct {
    const char* id;
    int32_t     index;

    IconType    icon;
    const char* label;

    UiSize w, h;
    float  pad_x, gap, radius;
    float  icon_size;      // 0 = derived from the label's font size

    Color  bg, bg_hover, fg, fg_hover;
} UiButtonDesc;

// Declares a button and returns true on the frame it is clicked. Hover state
// comes from the pointer position against last frame's bounds, which is why an
// id matters: without one the button cannot be tracked between frames.
bool uil_button(UiButtonDesc desc);
#define UI_BUTTON(...) uil_button((UiButtonDesc){ __VA_ARGS__ })

// ---------------------------------------------------------------------------
// Macros. These wrap the element-scope machinery; the trailing block holds the
// children.
// ---------------------------------------------------------------------------

void uil__open(int direction, UiBoxDesc desc);
void uil__close(void);

#define UIL__SCOPE(dir, ...)                                                    \
    for (int uil__once = (uil__open((dir), (UiBoxDesc){ __VA_ARGS__ }), 0);      \
         uil__once < 1;                                                         \
         ++uil__once, uil__close())

// Opens a box pinned to an absolute rect. Used to emit a docked panel's content
// inside the frame's single layout, at the rect the dock solved for it.
#define UI_PANEL(rect, ...)                                                    \
    UI_BOX(.floating = true, .float_to_root = true,                            \
           .float_attach = UI_ATTACH_TOP_LEFT,                                 \
           .float_offset = { (rect).x, (rect).y },                             \
           .w = UI_PX((rect).w), .h = UI_PX((rect).h),                         \
           __VA_ARGS__)

#define UI_ROW(...) UIL__SCOPE(0, __VA_ARGS__)
#define UI_COL(...) UIL__SCOPE(1, __VA_ARGS__)
#define UI_BOX(...) UIL__SCOPE(1, __VA_ARGS__)

#define UI_LABEL(text, ...)     uil_label((text), (UiTextDesc){ __VA_ARGS__ })
#define UI_PARAGRAPH(text, ...) uil_paragraph((text), (UiTextDesc){ __VA_ARGS__ })
#define UI_ICON(icon, size, col) uil_icon((icon), (float)(size), (col))
#define UI_SPACER(size)          uil_spacer(size)
#define UI_HOVERED()             uil_hovered()

#endif // UI_LAYOUT_H

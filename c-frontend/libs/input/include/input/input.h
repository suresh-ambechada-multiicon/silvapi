#ifndef INPUT_INPUT_H
#define INPUT_INPUT_H

#include "base/base.h"

// ---------------------------------------------------------------------------
// Input.
//
// Platform events are translated once, here, into a normalized per-frame event
// list plus queryable state. Nothing above this layer touches GLFW, and the
// core has no windowing dependency at all -- the platform adapter
// (input_glfw.c) is a separate translation unit.
//
// Two ways to consume input:
//
//   1. Polling, for immediate-mode widgets:
//          if (input_click_in(rect)) { ... }
//
//   2. Bindings, for anything that should read as "when X, call Y":
//          input_bind_key("send", INPUT_KEY_ENTER, INPUT_MOD_CTRL, on_send, NULL);
//          input_bind_click("logo", logo_rect, on_logo, NULL);
//      then once per frame:
//          input_dispatch();
//
// Bindings exist so keyboard shortcuts stop living inside a platform callback
// (where they cannot be listed, rebound, or tested) and become data.
// ---------------------------------------------------------------------------

// --- keys ------------------------------------------------------------------
// Platform-independent codes; the adapter maps from the native set.

typedef enum {
    INPUT_KEY_UNKNOWN = 0,

    INPUT_KEY_SPACE = 32,
    INPUT_KEY_COMMA = 44,
    INPUT_KEY_MINUS,
    INPUT_KEY_PERIOD,
    INPUT_KEY_SLASH,
    INPUT_KEY_0 = 48,
    INPUT_KEY_9 = 57,
    INPUT_KEY_A = 65,
    INPUT_KEY_B, INPUT_KEY_C, INPUT_KEY_D, INPUT_KEY_E, INPUT_KEY_F,
    INPUT_KEY_G, INPUT_KEY_H, INPUT_KEY_I, INPUT_KEY_J, INPUT_KEY_K,
    INPUT_KEY_L, INPUT_KEY_M, INPUT_KEY_N, INPUT_KEY_O, INPUT_KEY_P,
    INPUT_KEY_Q, INPUT_KEY_R, INPUT_KEY_S, INPUT_KEY_T, INPUT_KEY_U,
    INPUT_KEY_V, INPUT_KEY_W, INPUT_KEY_X, INPUT_KEY_Y, INPUT_KEY_Z,

    INPUT_KEY_ESCAPE = 256,
    INPUT_KEY_ENTER,
    INPUT_KEY_TAB,
    INPUT_KEY_BACKSPACE,
    INPUT_KEY_INSERT,
    INPUT_KEY_DELETE,
    INPUT_KEY_RIGHT,
    INPUT_KEY_LEFT,
    INPUT_KEY_DOWN,
    INPUT_KEY_UP,
    INPUT_KEY_PAGE_UP,
    INPUT_KEY_PAGE_DOWN,
    INPUT_KEY_HOME,
    INPUT_KEY_END,
    INPUT_KEY_F1, INPUT_KEY_F2, INPUT_KEY_F3, INPUT_KEY_F4,
    INPUT_KEY_F5, INPUT_KEY_F6, INPUT_KEY_F7, INPUT_KEY_F8,
    INPUT_KEY_F9, INPUT_KEY_F10, INPUT_KEY_F11, INPUT_KEY_F12,

    INPUT_KEY_COUNT = 512,
} InputKey;

typedef enum {
    INPUT_MOD_NONE  = 0,
    INPUT_MOD_SHIFT = 1 << 0,
    INPUT_MOD_CTRL  = 1 << 1,
    INPUT_MOD_ALT   = 1 << 2,
    INPUT_MOD_SUPER = 1 << 3,
} InputMod;

typedef enum {
    INPUT_MOUSE_LEFT = 0,
    INPUT_MOUSE_RIGHT,
    INPUT_MOUSE_MIDDLE,
    INPUT_MOUSE_COUNT,
} InputMouseButton;

// --- events ----------------------------------------------------------------

typedef enum {
    INPUT_EV_MOUSE_MOVE,
    INPUT_EV_MOUSE_DOWN,
    INPUT_EV_MOUSE_UP,
    INPUT_EV_SCROLL,
    INPUT_EV_KEY_DOWN,
    INPUT_EV_KEY_REPEAT,
    INPUT_EV_KEY_UP,
    INPUT_EV_TEXT,
} InputEventKind;

typedef struct {
    InputEventKind kind;
    Vec2     pos;          // pointer position when the event occurred
    Vec2     scroll;
    int      button;       // InputMouseButton
    int      key;          // InputKey
    uint32_t mods;         // InputMod bitmask
    uint32_t codepoint;    // INPUT_EV_TEXT
    int      click_count;  // 1 single, 2 double, 3 triple
    double   time;
} InputEvent;

// --- lifecycle -------------------------------------------------------------

void input_init(void);

// Clears per-frame edges (pressed/released/scroll/text) and the event list.
// Call before pumping platform events.
void input_begin_frame(double now_seconds);

// Called by the platform adapter. Also updates the queryable state.
void input_push(const InputEvent* ev);

// Tunables for double-click detection; defaults are 0.35s and 6 logical pixels.
void input_set_click_thresholds(double seconds, float distance_px);

// The timestamp passed to input_begin_frame(). Components that animate (a
// blinking caret, a hover fade) need a clock, and threading one through every
// call site is worse than reading it from the layer that already has it.
double input_time_seconds(void);

// --- event list ------------------------------------------------------------

size_t            input_event_count(void);
const InputEvent* input_event_at(size_t index);

// --- pointer state ---------------------------------------------------------

Vec2 input_mouse(void);
Vec2 input_mouse_delta(void);
Vec2 input_scroll(void);
int  input_click_count(void);

bool input_mouse_down(int button);
bool input_mouse_pressed(int button);   // went down this frame
bool input_mouse_released(int button);  // came up this frame

// Where the most recent press for `button` landed. Lets a widget require that
// press and release happen inside it, which is what a real click means.
Vec2 input_press_origin(int button);

// --- keyboard state --------------------------------------------------------

bool     input_key_down(int key);
bool     input_key_pressed(int key);    // includes auto-repeat
bool     input_key_released(int key);
uint32_t input_mods(void);

// UTF-8 text typed this frame, NUL-terminated. Empty string when none.
const char* input_text(void);

// --- hit testing -----------------------------------------------------------

bool input_hover(Rect r);
// Pressed inside `r` this frame.
bool input_press_in(Rect r, int button);
// Press and release both inside `r`: a completed click.
bool input_click_in(Rect r, int button);

// --- bindings --------------------------------------------------------------

typedef void (*InputAction)(void* user_data);

#define INPUT_MAX_BINDINGS 128

// `mods` must match exactly, so Ctrl+S does not also fire on Ctrl+Shift+S.
bool input_bind_key(const char* name, int key, uint32_t mods, InputAction fn, void* user);

// Click regions are re-registered each frame, since layout moves. Cleared by
// input_begin_frame().
bool input_bind_click(const char* name, Rect area, InputAction fn, void* user);

void input_unbind(const char* name);
void input_clear_key_bindings(void);

// Fires every binding whose condition is met this frame.
void input_dispatch(void);

// Introspection, so a UI can show or rebind shortcuts.
typedef struct {
    const char* name;
    int         key;
    uint32_t    mods;
    bool        is_click;
} InputBindingInfo;

size_t input_binding_count(void);
bool   input_binding_at(size_t index, InputBindingInfo* out);

// Renders a binding as text, e.g. "Ctrl+Shift+R". Returns `buf`.
const char* input_binding_label(const InputBindingInfo* info, char* buf, size_t cap);

#endif // INPUT_INPUT_H

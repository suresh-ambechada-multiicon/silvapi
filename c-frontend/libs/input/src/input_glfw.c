#include "input/input_glfw.h"
#include "GLFW/glfw3.h"

// GLFW's printable-key and function-key codes already match our numbering for
// ASCII and were chosen to match for the named keys, so most of this is a
// pass-through with an explicit table for the ones that differ.
int input_glfw_translate_key(int k) {
    if (k >= GLFW_KEY_A && k <= GLFW_KEY_Z) return INPUT_KEY_A + (k - GLFW_KEY_A);
    if (k >= GLFW_KEY_0 && k <= GLFW_KEY_9) return INPUT_KEY_0 + (k - GLFW_KEY_0);

    switch (k) {
        case GLFW_KEY_SPACE:     return INPUT_KEY_SPACE;
        case GLFW_KEY_COMMA:     return INPUT_KEY_COMMA;
        case GLFW_KEY_MINUS:     return INPUT_KEY_MINUS;
        case GLFW_KEY_PERIOD:    return INPUT_KEY_PERIOD;
        case GLFW_KEY_SLASH:     return INPUT_KEY_SLASH;
        case GLFW_KEY_ESCAPE:    return INPUT_KEY_ESCAPE;
        case GLFW_KEY_ENTER:     return INPUT_KEY_ENTER;
        case GLFW_KEY_KP_ENTER:  return INPUT_KEY_ENTER;
        case GLFW_KEY_TAB:       return INPUT_KEY_TAB;
        case GLFW_KEY_BACKSPACE: return INPUT_KEY_BACKSPACE;
        case GLFW_KEY_INSERT:    return INPUT_KEY_INSERT;
        case GLFW_KEY_DELETE:    return INPUT_KEY_DELETE;
        case GLFW_KEY_RIGHT:     return INPUT_KEY_RIGHT;
        case GLFW_KEY_LEFT:      return INPUT_KEY_LEFT;
        case GLFW_KEY_DOWN:      return INPUT_KEY_DOWN;
        case GLFW_KEY_UP:        return INPUT_KEY_UP;
        case GLFW_KEY_PAGE_UP:   return INPUT_KEY_PAGE_UP;
        case GLFW_KEY_PAGE_DOWN: return INPUT_KEY_PAGE_DOWN;
        case GLFW_KEY_HOME:      return INPUT_KEY_HOME;
        case GLFW_KEY_END:       return INPUT_KEY_END;
        case GLFW_KEY_F1:  return INPUT_KEY_F1;
        case GLFW_KEY_F2:  return INPUT_KEY_F2;
        case GLFW_KEY_F3:  return INPUT_KEY_F3;
        case GLFW_KEY_F4:  return INPUT_KEY_F4;
        case GLFW_KEY_F5:  return INPUT_KEY_F5;
        case GLFW_KEY_F6:  return INPUT_KEY_F6;
        case GLFW_KEY_F7:  return INPUT_KEY_F7;
        case GLFW_KEY_F8:  return INPUT_KEY_F8;
        case GLFW_KEY_F9:  return INPUT_KEY_F9;
        case GLFW_KEY_F10: return INPUT_KEY_F10;
        case GLFW_KEY_F11: return INPUT_KEY_F11;
        case GLFW_KEY_F12: return INPUT_KEY_F12;
        default:           return INPUT_KEY_UNKNOWN;
    }
}

static uint32_t translate_mods(int glfw_mods) {
    uint32_t m = INPUT_MOD_NONE;
    if (glfw_mods & GLFW_MOD_SHIFT)   m |= INPUT_MOD_SHIFT;
    if (glfw_mods & GLFW_MOD_CONTROL) m |= INPUT_MOD_CTRL;
    if (glfw_mods & GLFW_MOD_ALT)     m |= INPUT_MOD_ALT;
    if (glfw_mods & GLFW_MOD_SUPER)   m |= INPUT_MOD_SUPER;
    return m;
}

static int translate_button(int glfw_button) {
    switch (glfw_button) {
        case GLFW_MOUSE_BUTTON_LEFT:   return INPUT_MOUSE_LEFT;
        case GLFW_MOUSE_BUTTON_RIGHT:  return INPUT_MOUSE_RIGHT;
        case GLFW_MOUSE_BUTTON_MIDDLE: return INPUT_MOUSE_MIDDLE;
        default:                       return -1;
    }
}

static Vec2 cursor_of(GLFWwindow* w) {
    double x = 0.0, y = 0.0;
    glfwGetCursorPos(w, &x, &y);
    return (Vec2){ (float)x, (float)y };
}

static void on_cursor(GLFWwindow* w, double x, double y) {
    (void)w;
    InputEvent ev = { .kind = INPUT_EV_MOUSE_MOVE, .pos = { (float)x, (float)y } };
    input_push(&ev);
}

static void on_button(GLFWwindow* w, int button, int action, int mods) {
    int b = translate_button(button);
    if (b < 0) return;

    InputEvent ev = {
        .kind = (action == GLFW_PRESS) ? INPUT_EV_MOUSE_DOWN : INPUT_EV_MOUSE_UP,
        .pos = cursor_of(w),
        .button = b,
        .mods = translate_mods(mods),
        .time = glfwGetTime(),
    };
    input_push(&ev);
}

static void on_scroll(GLFWwindow* w, double dx, double dy) {
    InputEvent ev = {
        .kind = INPUT_EV_SCROLL,
        .pos = cursor_of(w),
        .scroll = { (float)dx, (float)dy },
    };
    input_push(&ev);
}

static void on_key(GLFWwindow* w, int key, int scancode, int action, int mods) {
    (void)scancode;
    int k = input_glfw_translate_key(key);
    if (k == INPUT_KEY_UNKNOWN) return;

    InputEventKind kind = (action == GLFW_PRESS)   ? INPUT_EV_KEY_DOWN
                        : (action == GLFW_REPEAT)  ? INPUT_EV_KEY_REPEAT
                                                   : INPUT_EV_KEY_UP;
    InputEvent ev = {
        .kind = kind,
        .pos = cursor_of(w),
        .key = k,
        .mods = translate_mods(mods),
        .time = glfwGetTime(),
    };
    input_push(&ev);
}

static void on_char(GLFWwindow* w, unsigned int codepoint) {
    (void)w;
    // Control characters arrive as key events, not text.
    if (codepoint < 0x20 || codepoint == 0x7F) return;
    InputEvent ev = { .kind = INPUT_EV_TEXT, .codepoint = codepoint };
    input_push(&ev);
}

void input_glfw_attach(GLFWwindow* window) {
    if (!window) return;
    glfwSetCursorPosCallback(window, on_cursor);
    glfwSetMouseButtonCallback(window, on_button);
    glfwSetScrollCallback(window, on_scroll);
    glfwSetKeyCallback(window, on_key);
    glfwSetCharCallback(window, on_char);
}

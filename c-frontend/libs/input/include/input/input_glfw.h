#ifndef INPUT_INPUT_GLFW_H
#define INPUT_INPUT_GLFW_H

#include "input/input.h"

// GLFW adapter, kept in its own translation unit so the input core stays free
// of any windowing dependency. Swapping to SDL or a native backend means adding
// a sibling file, not touching input.c or any caller.

struct GLFWwindow;

// Installs the cursor, button, scroll, key, and char callbacks on `window` and
// routes them into input_push().
void input_glfw_attach(struct GLFWwindow* window);

// Maps a native GLFW key code to an InputKey. Exposed for tests.
int input_glfw_translate_key(int glfw_key);

#endif // INPUT_INPUT_GLFW_H

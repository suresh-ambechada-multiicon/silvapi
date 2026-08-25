#ifndef UI_COMPONENTS_KBD_H
#define UI_COMPONENTS_KBD_H

#include "ui/layout.h"
#include "ui/style.h"

// ---------------------------------------------------------------------------
// Keybinding chips.
//
// Renders "Ctrl+Shift+P" as three separate chips rather than one string, which
// is what makes a shortcut scannable in a menu row. The separator is '+' and
// surrounding spaces are ignored, so "Ctrl + Shift + P" parses identically.
//
// A chord list uses spaces between alternatives: "MouseMiddle Ctrl+W Ctrl+F4"
// renders as three independent bindings, matching how a command with several
// bindings is shown.
// ---------------------------------------------------------------------------

typedef struct {
    const char* class;      // chip box overrides
    const char* text_class; // chip label overrides
    float       height;     // 0 = derived from the font
    bool        muted;      // dimmer treatment for a disabled row
} UiKbdProps;

// One binding: "Ctrl+Shift+P" -> [Ctrl][Shift][P]
void ui_kbd(const char* binding, UiKbdProps props);

// Space-separated alternatives, each rendered as its own binding.
void ui_kbd_list(const char* bindings, UiKbdProps props);

// Width the chips will occupy, so a caller can reserve a column for them.
float ui_kbd_measure(const char* binding, UiKbdProps props);

#endif // UI_COMPONENTS_KBD_H

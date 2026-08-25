#ifndef UI_COMPONENTS_MENU_H
#define UI_COMPONENTS_MENU_H

#include "ui/layout.h"
#include "ui/style.h"

// ---------------------------------------------------------------------------
// Menu / command palette / options list.
//
// One component, because these are all the same thing: a filterable list whose
// rows carry an icon, a label, an optional description, and an optional
// trailing control. Building them separately is how a codebase ends up with
// four keyboard-navigation implementations that disagree.
//
//   context menu     items, no filter, no descriptions
//   command palette  filter + sections + descriptions + shortcut chips
//   options panel    filter + trailing toggles/steppers/selects
//   header menu      items with checkmarks
//
// Filtering is a case-insensitive subsequence match over label and description,
// ranked so that earlier and more contiguous matches sort first. A SECTION or
// SEPARATOR whose following group filters away is hidden with it, so an empty
// heading never survives a search.
//
// Keyboard: Up/Down move over selectable rows only (skipping headings and
// disabled rows), Enter activates, Escape dismisses.
// ---------------------------------------------------------------------------

typedef enum {
    UI_MENU_ACTION = 0,
    UI_MENU_TOGGLE,      // trailing switch bound to `checked`
    UI_MENU_CHECK,       // trailing checkmark when `checked`
    UI_MENU_SUBMENU,     // trailing chevron; caller opens the child
    UI_MENU_VALUE,       // trailing read-only value, e.g. a select's current choice
    UI_MENU_CONTROL,     // caller emits the trailing control itself
    UI_MENU_SEPARATOR,
    UI_MENU_SECTION,     // non-selectable heading
} UiMenuItemKind;

typedef struct UiMenuItem {
    UiMenuItemKind kind;
    const char* id;
    const char* label;
    const char* description;   // second line, muted; NULL for a single-line row
    const char* shortcut;      // "Ctrl+E", or space-separated alternatives
    const char* value;         // UI_MENU_VALUE trailing text
    IconType    icon;
    bool*       checked;       // UI_MENU_TOGGLE / UI_MENU_CHECK
    bool        disabled;
    bool        starred;       // trailing star, for a favourites list
    void*       user;

    // UI_MENU_CONTROL: called in the row's trailing slot, inside the row's
    // layout, so whatever it emits is aligned with every other row's control.
    //
    // A slot rather than the menu knowing about sliders and steppers: an options
    // list, a submenu, and a command palette are all this one menu, and teaching
    // it every control would make it depend on all of them.
    // `struct UiMenuItem` rather than the typedef: the typedef name is not in
    // scope until this declaration closes.
    void (*trailing)(const struct UiMenuItem* item, void* user);
    float trailing_width;      // 0 = fit content
} UiMenuItem;

typedef struct {
    const char* id;

    // Style overrides merged over the component defaults.
    const char* class;
    const char* row_class;
    const char* filter_class;

    const UiMenuItem* items;
    size_t            item_count;

    // Filter box. NULL buffer disables it entirely (a context menu wants none).
    char*       filter_buffer;
    size_t      filter_buffer_size;
    const char* placeholder;

    float width;        // 0 = fit content
    float max_height;   // 0 = unbounded; beyond this the list scrolls
    float row_height;   // 0 = derived; two-line rows are taller automatically

    int* selected;      // keyboard cursor, index into `items`; -1 for none
    bool show_hints;    // footer with Up/Down/Enter/Escape chips
    bool autofocus;     // focus the filter box on the frame it appears
} UiMenuProps;

typedef struct {
    int    activated;      // index into items, -1 if none
    int    hovered;        // -1 if none
    int    toggled;        // index whose `checked` flipped this frame, -1
    bool   dismissed;      // Escape pressed
    size_t visible_count;  // rows surviving the filter
} UiMenuResult;

UiMenuResult ui_menu(const UiMenuProps* props);

// The filter ranking, exposed so a caller can sort its own data the same way.
// Returns -1 when `needle` does not match; higher is a better match.
int ui_menu_fuzzy_score(const char* haystack, const char* needle);

#endif // UI_COMPONENTS_MENU_H

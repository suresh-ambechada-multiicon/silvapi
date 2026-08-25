#include "ui/style.h"
#include "ui/theme.h"

// ---------------------------------------------------------------------------
// Theme -> style tokens.
//
// The only place that maps theme struct fields onto token names. Components
// reference "bg-surface" or "text-muted"; none of them know a Theme exists,
// which is what lets the component library drop into an application with a
// completely different theme representation -- that app just registers its own
// tokens under the same names.
// ---------------------------------------------------------------------------

void ui_style_load_theme_tokens(void) {
    Theme* t = theme_get_active();
    if (!t) return;

    // Surfaces
    ui_style_set_token("background",   t->background);
    ui_style_set_token("surface",      t->panel_bg);
    ui_style_set_token("sidebar",      t->sidebar_bg);
    ui_style_set_token("topbar",       t->topbar_bg);
    ui_style_set_token("tabbar",       t->tabbar_bg);
    ui_style_set_token("tab-active",   t->tab_active_bg);
    ui_style_set_token("tab-inactive", t->tab_inactive_bg);

    // Lines
    ui_style_set_token("border",        t->border);
    ui_style_set_token("border-subtle", t->border_subtle);

    // Text
    ui_style_set_token("fg",       t->foreground);
    ui_style_set_token("muted",    t->text_muted);
    ui_style_set_token("dim",      t->text_dim);
    ui_style_set_token("selected", t->text_selected);

    // Accents
    ui_style_set_token("primary",        t->primary);
    ui_style_set_token("primary-hover",  t->primary_hover);
    ui_style_set_token("primary-active", t->primary_active);
    ui_style_set_token("primary-fg",     t->primary_text);

    ui_style_set_token("secondary",        t->secondary);
    ui_style_set_token("secondary-hover",  t->secondary_hover);
    ui_style_set_token("secondary-active", t->secondary_active);
    ui_style_set_token("secondary-fg",     t->secondary_text);

    // shadcn-style aliases: "accent" is the subtle hover wash, which is what
    // most hover states in this UI actually want.
    ui_style_set_token("accent",        t->secondary_hover);
    ui_style_set_token("accent-active", t->secondary_active);

    ui_style_set_token("danger",       t->danger);
    ui_style_set_token("danger-hover", t->danger_hover);
    ui_style_set_token("success",      t->success);
    ui_style_set_token("warning",      t->warning);
    ui_style_set_token("info",         t->info);

    // Inputs
    ui_style_set_token("input",        t->input_bg);
    ui_style_set_token("input-border", t->input_border);
    ui_style_set_token("input-focus",  t->input_border_focus);
    ui_style_set_token("placeholder",  t->input_placeholder);

    // Tokens changed, so previously parsed styles hold stale colours.
    ui_style_invalidate();
}

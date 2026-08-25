// Standalone parser test. Linked without GL, so it exercises style.c only.
#include "ui/style.h"
#include <assert.h>

static int g_fail = 0;

static void check(bool cond, const char* what) {
    if (!cond) {
        fprintf(stderr, "FAIL: %s\n", what);
        g_fail++;
    }
}

static void check_f(float got, float want, const char* what) {
    if (fabsf(got - want) > 0.001f) {
        fprintf(stderr, "FAIL: %s (got %.3f want %.3f)\n", what, got, want);
        g_fail++;
    }
}

int main(void) {
    ui_style_set_spacing_unit(4.0f);
    ui_style_set_radius_scale(2.0f, 4.0f, 8.0f);
    ui_style_set_token("primary", color_hex(0x3B82F6));
    ui_style_set_token("surface", color_hex(0x111827));
    ui_style_set_token("accent",  color_hex(0x1F2937));
    ui_style_set_warn_unknown(true);

    // --- spacing scale ---
    const UiStyle* s = ui_style("p-2 gap-3 px-4");
    check_f(s->pad_t, 8.0f,  "p-2 -> 8px top");
    check_f(s->pad_l, 16.0f, "px-4 overrides horizontal to 16px");
    check_f(s->pad_b, 8.0f,  "p-2 survives for bottom");
    check_f(s->gap,   12.0f, "gap-3 -> 12px");

    // --- raw pixels vs scale ---
    s = ui_style("h-28px w-8");
    check(s->h.kind == UI_SIZE_FIXED && fabsf(s->h.value - 28.0f) < 0.001f, "h-28px is raw pixels");
    check(s->w.kind == UI_SIZE_FIXED && fabsf(s->w.value - 32.0f) < 0.001f, "w-8 is 8 * unit");

    // --- grow / fit / percent ---
    s = ui_style("w-full h-fit");
    check(s->w.kind == UI_SIZE_GROW, "w-full -> grow");
    check(s->h.kind == UI_SIZE_FIT,  "h-fit -> fit");

    s = ui_style("w-1/2");
    check(s->w.kind == UI_SIZE_PERCENT, "w-1/2 -> percent");
    check_f(s->w.value, 0.5f, "w-1/2 value");

    s = ui_style("w-min-40");
    check(s->w.kind == UI_SIZE_GROW, "w-min-40 grows");
    check_f(s->w.min, 160.0f, "w-min-40 minimum");

    // --- colour tokens and state fallback ---
    s = ui_style("bg-surface hover:bg-accent text-primary");
    Color base  = ui_style_bg(s, UI_STATE_BASE);
    Color hover = ui_style_bg(s, UI_STATE_HOVER);
    Color focus = ui_style_bg(s, UI_STATE_FOCUS);
    check_f(base.r,  color_hex(0x111827).r, "bg-surface base");
    check_f(hover.r, color_hex(0x1F2937).r, "hover:bg-accent applies to hover");
    check_f(focus.r, color_hex(0x111827).r, "unset state falls back to base");
    check_f(ui_style_fg(s, UI_STATE_HOVER).b, color_hex(0x3B82F6).b,
            "fg falls back to base for hover");

    // --- literal hex ---
    s = ui_style("bg-#ff0000");
    check_f(ui_style_bg(s, UI_STATE_BASE).r, 1.0f, "literal hex red");

    s = ui_style("bg-transparent");
    check_f(ui_style_bg(s, UI_STATE_BASE).a, 0.0f, "bg-transparent is clear");

    // --- radius, border ---
    s = ui_style("rounded-lg border-2");
    check_f(s->radius, 8.0f, "rounded-lg");
    check_f(s->border, 2.0f, "border-2 width");

    // A colour-only border class should imply a 1px width.
    s = ui_style("border-primary");
    check_f(s->border, 1.0f, "border-<token> implies 1px");

    // --- direction, alignment, text ---
    s = ui_style("col items-center justify-end font-bold truncate text-right");
    check(s->direction == 1, "col");
    check(s->align_y == UI_CENTER, "items-center");
    check(s->align_x == UI_END, "justify-end");
    check(s->font == UI_FONT_BOLD, "font-bold");
    check(s->truncate, "truncate");
    check(s->text_align == TEXT_ALIGN_RIGHT, "text-right");

    // --- overflow / float ---
    s = ui_style("scroll-y clip-x float-root at-bottom-center z-200");
    check(s->scroll_y && s->clip_x, "scroll-y + clip-x");
    check(s->floating && s->float_to_root, "float-root");
    check(s->float_attach == UI_ATTACH_BOTTOM_CENTER, "at-bottom-center");
    check(s->float_z == 200, "z-200 is a raw z index");

    // --- set_mask semantics ---
    s = ui_style("h-8");
    check((s->set_mask & UI_STYLE_SET_H) != 0, "h-8 marks H set");
    check((s->set_mask & UI_STYLE_SET_W) == 0, "w untouched");

    // --- merge: overrides win, unspecified fields survive ---
    const UiStyle* m = ui_style_merge("h-8 px-3 bg-surface", "bg-primary");
    check_f(m->pad_l, 12.0f, "merge keeps base padding");
    check_f(m->h.value, 32.0f, "merge keeps base height");
    check_f(ui_style_bg(m, UI_STATE_BASE).b, color_hex(0x3B82F6).b, "merge override wins on bg");

    // --- cache behaviour ---
    (void)ui_style("h-8 px-3 bg-surface");        // warm the entry
    UiStyleStats before = ui_style_stats();
    for (int i = 0; i < 1000; i++) (void)ui_style("h-8 px-3 bg-surface");
    UiStyleStats after = ui_style_stats();
    check(after.parses == before.parses, "repeat lookups do not reparse");
    check(after.hits > before.hits, "repeat lookups hit the cache");

    // --- theme change invalidates ---
    ui_style_set_token("surface", color_hex(0xFFFFFF));
    s = ui_style("bg-surface");
    check_f(ui_style_bg(s, UI_STATE_BASE).r, 1.0f, "token change is picked up");

    // --- unknown classes are reported, not silently ignored ---
    UiStyleStats u0 = ui_style_stats();
    (void)ui_style("totally-not-a-class");
    check(ui_style_stats().unknown_classes > u0.unknown_classes, "unknown class counted");

    // --- box/text projection ---
    s = ui_style("row h-8 px-3 gap-2 rounded-md bg-surface hover:bg-accent text-primary");
    UiBoxDesc box = ui_style_box(s, "btn", 0, UI_STATE_HOVER);
    check(box.id && strcmp(box.id, "btn") == 0, "box carries id");
    check_f(box.pad_l, 12.0f, "box padding");
    check_f(box.gap, 8.0f, "box gap");
    check_f(box.bg.r, color_hex(0x1F2937).r, "box uses hover bg");
    UiTextDesc td = ui_style_text(s, UI_STATE_BASE);
    check_f(td.color.b, color_hex(0x3B82F6).b, "text desc colour");

    // Regression: a two-character token name in a longer class list. The table
    // component uses exactly this and rendered invisible text.
    ui_style_set_token("fg", color_hex(0xE6E6E6));
    {
        const UiStyle* r2 = ui_style("row w-full items-center text-fg");
        Color f = ui_style_fg(r2, UI_STATE_BASE);
        check(f.a > 0.5f, "text-fg inside a class list resolves");

        const UiStyle* r3 = ui_style("text-fg");
        check(ui_style_fg(r3, UI_STATE_BASE).a > 0.5f, "text-fg alone resolves");

        Color got;
        check(ui_style_get_token("fg", &got), "token fg is retrievable");
    }

    if (g_fail == 0) {
        printf("style: all checks passed\n");
        return 0;
    }
    printf("style: %d FAILURES\n", g_fail);
    return 1;
}

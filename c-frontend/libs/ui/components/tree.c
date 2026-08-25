#include "ui/components/tree.h"
#include "input/input.h"

#define TREE_CLASS_DEFAULT "col w-full h-full"
#define TREE_ROW_DEFAULT   "row items-center px-2 gap-1 rounded-sm text-fg"
#define TREE_MAX_DEPTH     32

// Retained per-tree state. Scroll is owned here rather than by Clay because
// culling needs the offset BEFORE deciding which rows to emit.
typedef struct {
    uint64_t key;
    float    scroll_y;
    int      last_click_row;
    bool     used;
} TreeState;

#define TREE_STATE_SLOTS 16
static TreeState s_states[TREE_STATE_SLOTS];

static uint64_t tree_key(const char* id) {
    uint64_t h = 1469598103934665603ULL;
    for (const char* p = id ? id : "tree"; *p; p++) {
        h ^= (uint8_t)*p;
        h *= 1099511628211ULL;
    }
    return h ? h : 1;
}

static TreeState* tree_state(const char* id) {
    uint64_t key = tree_key(id);
    for (size_t i = 0; i < TREE_STATE_SLOTS; i++) {
        if (s_states[i].used && s_states[i].key == key) return &s_states[i];
    }
    for (size_t i = 0; i < TREE_STATE_SLOTS; i++) {
        if (!s_states[i].used) {
            TreeState* st = &s_states[i];
            memset(st, 0, sizeof(*st));
            st->key = key;
            st->used = true;
            st->last_click_row = -1;
            return st;
        }
    }
    return NULL;
}

// ---------------------------------------------------------------------------
// Active tree, valid between begin and end. Only one tree is built at a time,
// which is what lets ui_tree_node() be a free function with no handle.
// ---------------------------------------------------------------------------

typedef struct {
    bool         active;
    UiTreeProps  props;
    TreeState*   state;
    const UiStyle* row_style;

    float row_h;
    float indent;

    int   depth;
    int   row;            // ordinal of the row about to be emitted
    char  row_id[96];     // namespaced per instance; see ui_tree_begin
    float view_top;       // scroll offset in row space
    float view_h;
    Rect  viewport;

    // Visible rows live inside one floating container offset by the scroll
    // residual. A leading spacer cannot express that offset: the residual is
    // negative whenever the top row is only partially scrolled into view, and
    // clamping it to zero makes the list snap row-by-row instead of scrolling.
    bool  rows_open;

    UiTreeResult result;

    // Per-node interaction, reset by each ui_tree_node().
    bool node_hovered, node_clicked, node_activated, node_context, node_toggled;

    // Theme colours resolved once per tree rather than per row.
    Color fg, muted, accent, primary, primary_fg, guide, stripe;
} TreeCtx;

static TreeCtx g_tree;

void ui_tree_begin(const UiTreeProps* props) {
    memset(&g_tree, 0, sizeof(g_tree));
    if (!props) return;

    g_tree.active = true;
    g_tree.props = *props;
    g_tree.state = tree_state(props->id);
    g_tree.row_style = ui_style_merge(TREE_ROW_DEFAULT, props->row_class);
    g_tree.row_h = props->row_height > 0.0f ? props->row_height : 26.0f;
    g_tree.indent = props->indent > 0.0f ? props->indent : 14.0f;
    g_tree.result = (UiTreeResult){ .clicked = -1, .activated = -1,
                                    .context = -1, .toggled = -1 };

    g_tree.fg = ui_style_fg(g_tree.row_style, UI_STATE_BASE);
    if (!ui_style_get_token("muted", &g_tree.muted))          g_tree.muted = g_tree.fg;
    if (!ui_style_get_token("accent", &g_tree.accent))        g_tree.accent = (Color){0};
    if (!ui_style_get_token("primary", &g_tree.primary))      g_tree.primary = g_tree.accent;
    if (!ui_style_get_token("primary-fg", &g_tree.primary_fg)) g_tree.primary_fg = g_tree.fg;
    if (!ui_style_get_token("border-subtle", &g_tree.guide))  g_tree.guide = g_tree.muted;
    g_tree.stripe = color_with_alpha(g_tree.fg, 0.03f);

    // Namespaced per instance: two trees in one layout would otherwise declare
    // the same row ids and their hit-testing would cross-talk.
    snprintf(g_tree.row_id, sizeof(g_tree.row_id), "%s.row",
             props->id ? props->id : "tree");

    char vp_id[96];
    snprintf(vp_id, sizeof(vp_id), "%s.vp", props->id ? props->id : "tree");
    g_tree.viewport = uil_element_rect(vp_id, 0);
    g_tree.view_h = g_tree.viewport.h > 0.0f ? g_tree.viewport.h
                                             : (props->max_height > 0.0f ? props->max_height
                                                                        : g_tree.row_h * 20.0f);

    TreeState* st = g_tree.state;
    if (st) {
        // Scrolling only responds while the pointer is over the viewport, so a
        // tree does not steal wheel events from a neighbouring panel.
        if (g_tree.viewport.h > 0.0f && input_hover(g_tree.viewport)) {
            Vec2 sc = input_scroll();
            if (sc.y != 0.0f) st->scroll_y -= sc.y * g_tree.row_h * 3.0f;
        }
        if (st->scroll_y < 0.0f) st->scroll_y = 0.0f;
        g_tree.view_top = st->scroll_y;
    }

    const UiStyle* ts = ui_style_merge(TREE_CLASS_DEFAULT, props->class);
    UiBoxDesc outer = ui_style_box(ts, vp_id, 0, UI_STATE_BASE);
    outer.w = UI_GROW;
    outer.h = props->max_height > 0.0f ? UI_PX(props->max_height) : UI_GROW;
    outer.clip_y = true;
    outer.clip_x = true;

    uil__open(1 /* col */, outer);
}

// Opens the row container on the first visible row, positioned so that row lands
// at its true scrolled offset. Culled rows before it are simply never emitted;
// the scrollable extent comes from the row count, not from laid-out height.
static void open_rows(float first_row_top) {
    if (g_tree.rows_open) return;
    g_tree.rows_open = true;

    float offset = first_row_top - g_tree.view_top;

    UiBoxDesc rows = {
        .w = UI_GROW,
        .h = UI_FIT,
        .floating = true,
        .float_attach = UI_ATTACH_TOP_LEFT,
        .float_clip_to_parent = true,
        .float_offset = { 0.0f, offset },
    };
    uil__open(1 /* col */, rows);
}

bool ui_tree_node(const UiTreeNode* n) {
    g_tree.node_hovered = g_tree.node_clicked = false;
    g_tree.node_activated = g_tree.node_context = g_tree.node_toggled = false;

    if (!g_tree.active || !n) return false;

    int row = g_tree.row++;
    g_tree.result.row_count++;

    // Cull rows outside the viewport, with one row of overscan either side so a
    // partially scrolled row is present rather than popping in.
    float row_top = (float)row * g_tree.row_h;
    float rel_top = row_top - g_tree.view_top;
    if (rel_top + g_tree.row_h < -g_tree.row_h || rel_top > g_tree.view_h + g_tree.row_h) {
        if (n->expandable && n->expanded) {
            g_tree.depth++;
            return true;
        }
        return false;
    }
    open_rows(row_top);
    g_tree.result.emitted++;

    bool hovered = uil_element_hovered(g_tree.row_id, row);
    bool selected = n->selected ||
                    (g_tree.props.selected && *g_tree.props.selected == row);

    g_tree.node_hovered = hovered;

    Color row_bg = selected ? g_tree.primary
                 : hovered  ? g_tree.accent
                 : (g_tree.props.striped && (row & 1)) ? g_tree.stripe
                                                       : (Color){0};
    Color label_fg = n->disabled ? g_tree.muted
                   : selected    ? g_tree.primary_fg
                                 : g_tree.fg;

    float pad_l = 6.0f + g_tree.indent * (float)g_tree.depth;

    UI_ROW(.id = g_tree.row_id, .index = row,
           .w = UI_GROW, .h = UI_PX(g_tree.row_h),
           .pad_l = pad_l, .pad_r = 6.0f, .gap = 4.0f,
           .align_y = UI_CENTER, .radius = 3.0f, .bg = row_bg) {

        // Indent guides: one rule per ancestor level, drawn as layout boxes so
        // they cannot drift out of alignment with the indentation.
        if (g_tree.props.show_guides) {
            for (int d = 0; d < g_tree.depth; d++) {
                UI_BOX(.w = UI_PX(1.0f), .h = UI_PCT(1.0f), .bg = g_tree.guide) {}
                UI_SPACER(UI_PX(g_tree.indent - 5.0f));
            }
        }

        // The chevron slot is reserved even for leaves, so labels line up
        // whether or not a sibling can expand.
        if (n->expandable) {
            UI_ICON(n->expanded ? ICON_CHEVRON_DOWN : ICON_CHEVRON_RIGHT,
                    12.0f, selected ? g_tree.primary_fg : g_tree.muted);
        } else {
            UI_SPACER(UI_PX(12.0f));
        }

        if (n->badge && *n->badge) {
            // On a selected row the badge sits on the primary fill, where a
            // low-alpha tint of its own colour washes out and the text becomes
            // unreadable. Selected rows tint the badge with the row's
            // foreground instead.
            Color tint = selected ? g_tree.primary_fg
                       : (n->badge_color.a > 0.0f ? n->badge_color : g_tree.muted);
            UI_ROW(.h = UI_PX(17.0f), .pad_x = 5.0f, .align_y = UI_CENTER,
                   .radius = 3.0f, .bg = color_with_alpha(tint, selected ? 0.22f : 0.16f)) {
                UI_LABEL(n->badge, .color = tint, .font = UI_FONT_BOLD);
            }
        }

        if (n->icon != ICON_NONE) {
            UI_ICON(n->icon, 14.0f,
                    n->icon_color.a > 0.0f ? n->icon_color : label_fg);
        }

        UI_LABEL(n->label ? n->label : "", .ellipsize = true, .color = label_fg);

        if (n->value && *n->value) {
            UI_LABEL(n->value, .color = selected ? g_tree.primary_fg : g_tree.muted);
        }
        if (n->dot_color.a > 0.0f) {
            UI_BOX(.w = UI_PX(6.0f), .h = UI_PX(6.0f), .radius = 3.0f,
                   .bg = n->dot_color) {}
        }
    }

    if (hovered && !n->disabled) {
        if (input_mouse_pressed(INPUT_MOUSE_LEFT)) {
            g_tree.node_clicked = true;
            g_tree.result.clicked = row;
            if (g_tree.props.selected) *g_tree.props.selected = row;

            // An expandable row toggles on click; a leaf activates.
            if (n->expandable) {
                g_tree.node_toggled = true;
                g_tree.result.toggled = row;
            }

            TreeState* st = g_tree.state;
            if (input_click_count() >= 2 && st && st->last_click_row == row) {
                g_tree.node_activated = true;
                g_tree.result.activated = row;
            }
            if (st) st->last_click_row = row;
        }
        if (input_mouse_pressed(INPUT_MOUSE_RIGHT)) {
            g_tree.node_context = true;
            g_tree.result.context = row;
            if (g_tree.props.selected) *g_tree.props.selected = row;
        }
    }

    if (n->expandable && n->expanded) {
        // Balanced by the caller's ui_tree_pop().
        if (g_tree.depth < TREE_MAX_DEPTH) g_tree.depth++;
        return true;
    }
    return false;
}

void ui_tree_pop(void) {
    if (!g_tree.active) return;
    if (g_tree.depth > 0) g_tree.depth--;
}

UiTreeResult ui_tree_end(void) {
    if (!g_tree.active) return (UiTreeResult){ .clicked = -1, .activated = -1,
                                               .context = -1, .toggled = -1 };
    if (g_tree.rows_open) uil__close();   // row container
    uil__close();                          // viewport

    // Clamp the scroll now that the true content height is known.
    TreeState* st = g_tree.state;
    if (st) {
        float content_h = (float)g_tree.result.row_count * g_tree.row_h;
        float max_scroll = content_h > g_tree.view_h ? content_h - g_tree.view_h : 0.0f;
        if (st->scroll_y > max_scroll) st->scroll_y = max_scroll;
        if (st->scroll_y < 0.0f) st->scroll_y = 0.0f;
    }

    UiTreeResult r = g_tree.result;
    g_tree.active = false;
    return r;
}

bool ui_tree_node_clicked(void)   { return g_tree.node_clicked; }
bool ui_tree_node_activated(void) { return g_tree.node_activated; }
bool ui_tree_node_context(void)   { return g_tree.node_context; }
bool ui_tree_node_toggled(void)   { return g_tree.node_toggled; }
bool ui_tree_node_hovered(void)   { return g_tree.node_hovered; }

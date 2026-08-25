#include "app/views.h"
#include "app/config.h"
#include "ui/layout.h"
#include "ui/style.h"
#include "ui/components/table.h"
#include "ui/components/menu.h"
#include "ui/components/tree.h"
#include "ui/components/tabs.h"
#include "ui/components/code_editor.h"
#include "ui/components/switch.h"
#include "ui/components/slider.h"
#include "ui/components/stepper.h"
#include "ui/components/select.h"
#include "ui/components/breadcrumb.h"
#include "json/json_utils.h"
#include "ui/components/kbd.h"
#include "input/input.h"

// ---------------------------------------------------------------------------
// Component gallery.
//
// Exists so a component can be exercised in isolation, at scale, without
// depending on application data. Run with SILVAPI_DEMO=table.
//
// The row count is deliberately large: it is the only way to see whether
// virtualization actually holds, since a table that renders every row looks
// identical to one that does not until the row count gets big.
// ---------------------------------------------------------------------------

#define DEMO_ROWS 5000

static const char* const DEMO_KINDS[] = {
    "Local Disk", "Removable", "Network", "Optical",
};
static const char* const DEMO_FS[] = { "NTFS", "FAT32", "exFAT", "ext4" };

static void demo_cell(size_t row, size_t col, UiCell* out, void* user) {
    (void)user;

    // Deterministic pseudo-data: the same row always renders the same values, so
    // scrolling back and forth does not appear to change the contents.
    uint32_t h = (uint32_t)row * 2654435761u;
    float pct = (float)((h >> 8) % 101) / 100.0f;

    static char buf[64];   // one cell is consumed before the next call
    switch (col) {
        case 0:
            snprintf(buf, sizeof(buf), "Volume %zu (%c:)", row, (char)('C' + (row % 24)));
            out->kind = UI_CELL_ICON_TEXT;
            out->icon = ICON_DATABASE;
            out->text = buf;
            break;

        case 1:
            out->kind = UI_CELL_TEXT;
            out->text = DEMO_KINDS[row % 4];
            out->dim = true;
            break;

        case 2:
            out->kind = UI_CELL_BADGE;
            out->text = DEMO_FS[(h >> 4) % 4];
            out->color = (row % 3 == 0) ? color_hex(0x4EC9B0) : color_hex(0x569CD6);
            break;

        case 3:
            snprintf(buf, sizeof(buf), "%.1f GB", (double)((h >> 12) % 5000) / 10.0);
            out->kind = UI_CELL_TEXT;
            out->text = buf;
            break;

        case 4: {
            snprintf(buf, sizeof(buf), "%d%%", (int)(pct * 100.0f));
            out->kind = UI_CELL_PROGRESS;
            out->value = pct;
            out->text = buf;
            out->color = pct > 0.9f ? color_hex(0xF14C4C)
                       : pct > 0.7f ? color_hex(0xCCA700)
                                    : color_hex(0x3794FF);
            break;
        }

        default:
            out->kind = UI_CELL_TEXT;
            out->text = "";
            break;
    }
}

static UiTableColumn s_demo_cols[] = {
    { .id = "name",  .title = "Name",        .width = 260, .min_width = 120,
      .resizable = true, .sortable = true, .visible = true, .grow = true },
    { .id = "type",  .title = "Type",        .width = 110, .resizable = true,
      .sortable = true, .visible = true },
    { .id = "fs",    .title = "File System", .width = 120, .resizable = true,
      .sortable = true, .visible = true },
    { .id = "size",  .title = "Size",        .width = 100, .resizable = true,
      .sortable = true, .visible = true, .align = TEXT_ALIGN_RIGHT },
    { .id = "full",  .title = "Percent Full",.width = 220, .resizable = true,
      .visible = true, .grow = true },
};

static int  s_demo_sel = -1;
static int  s_demo_sort = -1;
static bool s_demo_desc = false;

void view_demo_gallery(Rect rect) {
    Theme* t = theme_get_active();

    UI_PANEL(rect, .bg = t->background) {
        UI_COL(.w = UI_GROW, .h = UI_GROW, .pad = 0.0f, .gap = 0.0f) {

            // Toolbar, entirely class-styled: no explicit colours at this call site.
            UI_ROW(.w = UI_GROW, .h = UI_PX(34.0f), .pad_x = 10.0f, .gap = 8.0f,
                   .align_y = UI_CENTER, .bg = t->panel_bg) {
                UI_LABEL("Component Gallery — Data Table", .font = UI_FONT_BOLD,
                         .color = t->foreground);
                UI_SPACER(UI_GROW);

                char info[128];
                snprintf(info, sizeof(info), "%d rows", DEMO_ROWS);
                UI_LABEL(info, .color = t->text_muted);

                if (UI_BUTTON(.id = "demo.autosize", .icon = ICON_EXPAND,
                              .label = "Size Columns", .icon_size = 13.0f,
                              .h = UI_PX(24.0f), .radius = g_config.corner_radius,
                              .fg = t->text_muted, .fg_hover = t->foreground,
                              .bg_hover = t->secondary_hover)) {
                    ui_table_autosize_columns(s_demo_cols,
                        sizeof(s_demo_cols) / sizeof(s_demo_cols[0]),
                        demo_cell, NULL, 40);
                }
            }

            UiTableResult r = ui_table(&(UiTableProps){
                .id = "demo.table",
                .columns = s_demo_cols,
                .column_count = sizeof(s_demo_cols) / sizeof(s_demo_cols[0]),
                .row_count = DEMO_ROWS,
                .cell = demo_cell,
                .striped = true,
                .row_dividers = false,
                .selected_row = &s_demo_sel,
                .sort_column = &s_demo_sort,
                .sort_desc = &s_demo_desc,
            });

            // Status bar, mirroring what the table reports about its own window.
            UI_ROW(.w = UI_GROW, .h = UI_PX(26.0f), .pad_x = 10.0f, .gap = 12.0f,
                   .align_y = UI_CENTER, .bg = t->panel_bg) {
                char status[192];
                snprintf(status, sizeof(status),
                         "visible %zu..%zu (%zu emitted)   selected %d   sort col %d%s",
                         r.first_visible_row,
                         r.first_visible_row + r.visible_row_count,
                         r.visible_row_count, s_demo_sel, s_demo_sort,
                         s_demo_desc ? " desc" : "");
                UI_LABEL(status, .color = t->text_muted, .font = UI_FONT_MONO);
            }
        }
    }

    if (r_clip_depth() < 0) { /* unreachable; keeps r_clip_depth linked for tests */ }
}


// ---------------------------------------------------------------------------
// Menu demo: the three shapes the component has to cover, side by side, so a
// change that helps one and breaks another is visible immediately.
// ---------------------------------------------------------------------------

static char s_palette_filter[128];
static int  s_palette_sel = 0;

static char s_options_filter[128];
static int  s_options_sel = 0;

static int  s_ctx_sel = 0;

static bool s_opt_animations   = true;
static bool s_opt_round        = true;
static bool s_opt_system_theme = true;
static bool s_opt_alt_rows     = false;
static bool s_opt_hidden       = false;
static bool s_opt_ext          = true;

static bool s_col_name = true;
static bool s_col_type = true;
static bool s_col_size = true;

static const UiMenuItem PALETTE_ITEMS[] = {
    { .kind = UI_MENU_SECTION, .label = "Quick", .icon = ICON_SPARKLES },
    { .kind = UI_MENU_ACTION, .id = "expand", .label = "Toggle Expand Folder",
      .description = "Recursively flatten all subfiles and subfolders into a single listing.",
      .shortcut = "Ctrl+E", .starred = true },
    { .kind = UI_MENU_ACTION, .id = "inspector", .label = "Toggle Inspector",
      .description = "Toggle inspector in currently active panel.",
      .shortcut = "Space" },
    { .kind = UI_MENU_ACTION, .id = "split-right", .label = "Split Right",
      .description = "Split current panel to the right.",
      .shortcut = "Ctrl+Shift+S" },
    { .kind = UI_MENU_ACTION, .id = "split-bottom", .label = "Split Bottom",
      .description = "Split current panel to the bottom.",
      .shortcut = "Ctrl+S" },
    { .kind = UI_MENU_SEPARATOR },
    { .kind = UI_MENU_ACTION, .id = "new-tab", .label = "Open New Tab",
      .description = "Open a new tab in the same directory.",
      .shortcut = "Ctrl+T" },
    { .kind = UI_MENU_ACTION, .id = "close-tab", .label = "Close Tab",
      .description = "Close the current tab.",
      .shortcut = "MouseMiddle Ctrl+W Ctrl+F4" },
    { .kind = UI_MENU_ACTION, .id = "options", .label = "Options",
      .description = "Open the options window.", .shortcut = "Ctrl+," },
    { .kind = UI_MENU_ACTION, .id = "console", .label = "Open Console",
      .description = "Open the OS native console at the current path." },
    { .kind = UI_MENU_ACTION, .id = "disabled", .label = "Load Layout",
      .description = "No saved layout available.", .disabled = true },
};

static const UiMenuItem OPTION_ITEMS[] = {
    { .kind = UI_MENU_VALUE,  .label = "Font",
      .description = "The system font applied to the entire UI.", .value = "Adwaita Sans" },
    { .kind = UI_MENU_VALUE,  .label = "Font Size",
      .description = "Determines the scale of the whole UI.", .value = "15" },
    { .kind = UI_MENU_TOGGLE, .label = "Animations",
      .description = "Applies animations to popups, scrolling, and the text cursor.",
      .checked = &s_opt_animations },
    { .kind = UI_MENU_TOGGLE, .label = "Round Corners",
      .description = "Applies rounding to popups, buttons, and selections.",
      .checked = &s_opt_round },
    { .kind = UI_MENU_TOGGLE, .label = "System Light / Dark Scheme",
      .description = "Follow the system colour scheme automatically.",
      .checked = &s_opt_system_theme },
    { .kind = UI_MENU_VALUE,  .label = "Dark Color Scheme",
      .description = "Theme applied when the system uses dark colours.", .value = "Ayu Dark" },
    { .kind = UI_MENU_TOGGLE, .label = "Alternating Row Colors",
      .description = "Improves readability by distinguishing adjacent rows.",
      .checked = &s_opt_alt_rows },
    { .kind = UI_MENU_TOGGLE, .label = "Show Hidden Files",
      .checked = &s_opt_hidden },
    { .kind = UI_MENU_TOGGLE, .label = "Show File Extension",
      .checked = &s_opt_ext },
};

static const UiMenuItem CTX_ITEMS[] = {
    { .kind = UI_MENU_ACTION, .label = "Shift Column Left",  .icon = ICON_CHEVRON_LEFT },
    { .kind = UI_MENU_ACTION, .label = "Shift Column Right", .icon = ICON_CHEVRON_RIGHT,
      .disabled = true },
    { .kind = UI_MENU_ACTION, .label = "Size Column To Fit", .icon = ICON_EXPAND },
    { .kind = UI_MENU_ACTION, .label = "Close Column",       .icon = ICON_CLOSE,
      .shortcut = "MouseMiddle" },
    { .kind = UI_MENU_ACTION, .label = "Size All To Fit",    .icon = ICON_EXPAND,
      .shortcut = "Ctrl+=" },
    { .kind = UI_MENU_SEPARATOR },
    { .kind = UI_MENU_CHECK, .label = "Name", .checked = &s_col_name, .disabled = true },
    { .kind = UI_MENU_CHECK, .label = "Type", .checked = &s_col_type },
    { .kind = UI_MENU_CHECK, .label = "Size", .checked = &s_col_size },
};

void view_demo_menus(Rect rect) {
    Theme* t = theme_get_active();
    static int s_reported;

    UI_PANEL(rect, .bg = t->background) {
        UI_COL(.w = UI_GROW, .h = UI_GROW, .pad = 16.0f, .gap = 12.0f) {

            UI_ROW(.w = UI_GROW, .h = UI_PX(24.0f), .align_y = UI_CENTER, .gap = 8.0f) {
                UI_LABEL("Component Gallery — Menu / Palette / Options",
                         .font = UI_FONT_BOLD, .color = t->foreground);
                UI_SPACER(UI_GROW);
                UI_LABEL("filter, arrows, enter", .color = t->text_muted);
            }

            UI_ROW(.w = UI_GROW, .h = UI_GROW, .gap = 16.0f, .align_y = UI_START) {

                UI_COL(.gap = 6.0f) {
                    UI_LABEL("Command palette", .color = t->text_muted);
                    ui_menu(&(UiMenuProps){
                        .id = "demo.palette",
                        .items = PALETTE_ITEMS,
                        .item_count = sizeof(PALETTE_ITEMS) / sizeof(PALETTE_ITEMS[0]),
                        .filter_buffer = s_palette_filter,
                        .filter_buffer_size = sizeof(s_palette_filter),
                        .placeholder = "Select a command...",
                        .width = 470.0f,
                        .max_height = 420.0f,
                        .selected = &s_palette_sel,
                        .show_hints = true,
                    });
                    if (getenv("SILVAPI_MENU_TRACE") && ++s_reported % 30 == 0) {
                        fprintf(stderr, "[Menu] palette cursor=%d filter=\"%s\"\n",
                                s_palette_sel, s_palette_filter);
                    }
                }

                UI_COL(.gap = 6.0f) {
                    UI_LABEL("Options", .color = t->text_muted);
                    ui_menu(&(UiMenuProps){
                        .id = "demo.options",
                        .items = OPTION_ITEMS,
                        .item_count = sizeof(OPTION_ITEMS) / sizeof(OPTION_ITEMS[0]),
                        .filter_buffer = s_options_filter,
                        .filter_buffer_size = sizeof(s_options_filter),
                        .placeholder = "Select an option...",
                        .width = 460.0f,
                        .max_height = 420.0f,
                        .selected = &s_options_sel,
                    });
                }

                UI_COL(.gap = 6.0f) {
                    UI_LABEL("Context menu", .color = t->text_muted);
                    ui_menu(&(UiMenuProps){
                        .id = "demo.ctx",
                        .items = CTX_ITEMS,
                        .item_count = sizeof(CTX_ITEMS) / sizeof(CTX_ITEMS[0]),
                        .width = 250.0f,
                        .selected = &s_ctx_sel,
                    });
                }
            }
        }
    }
}


// ---------------------------------------------------------------------------
// Tree demo: a deep synthetic hierarchy, so depth, indent guides, and row
// culling are all exercised. Expansion state lives in a flat bitmap keyed by a
// path hash, which is how a real caller with no node objects would do it.
// ---------------------------------------------------------------------------

#define TREE_DEMO_FANOUT 6
#define TREE_DEMO_DEPTH  4
#define TREE_EXPAND_SLOTS 4096

static uint8_t s_expanded[TREE_EXPAND_SLOTS];
static int     s_tree_demo_sel = -1;
static bool    s_tree_guides = true;

static uint32_t path_hash(const int* path, int depth) {
    uint32_t h = 2166136261u;
    for (int i = 0; i < depth; i++) {
        h ^= (uint32_t)(path[i] + 1);
        h *= 16777619u;
    }
    return h % TREE_EXPAND_SLOTS;
}

static void emit_demo_subtree(int* path, int depth) {
    Theme* t = theme_get_active();

    for (int i = 0; i < TREE_DEMO_FANOUT; i++) {
        path[depth] = i;
        uint32_t key = path_hash(path, depth + 1);
        bool leaf = (depth + 1 >= TREE_DEMO_DEPTH);

        char label[96];
        if (leaf) snprintf(label, sizeof(label), "item-%d-%d.json", depth, i);
        else      snprintf(label, sizeof(label), "group-%d-%d", depth, i);

        char value[32];
        snprintf(value, sizeof(value), "%d KB", (int)(key % 900) + 1);

        bool open = ui_tree_node(&(UiTreeNode){
            .id = label,
            .label = label,
            .icon = leaf ? ICON_FILE_CODE : (s_expanded[key] ? ICON_FOLDER_OPEN : ICON_FOLDER),
            .icon_color = leaf ? t->info : t->warning,
            .expandable = !leaf,
            .expanded = s_expanded[key] != 0,
            .value = leaf ? value : NULL,
            .dot_color = (leaf && (key % 5) == 0) ? t->success : (Color){0},
        });

        if (ui_tree_node_toggled()) s_expanded[key] = !s_expanded[key];

        if (open) {
            emit_demo_subtree(path, depth + 1);
            ui_tree_pop();
        }
    }
}

void view_demo_tree(Rect rect) {
    Theme* t = theme_get_active();

    UI_PANEL(rect, .bg = t->background) {
        UI_COL(.w = UI_GROW, .h = UI_GROW, .pad = 12.0f, .gap = 8.0f) {

            UI_ROW(.w = UI_GROW, .h = UI_PX(26.0f), .align_y = UI_CENTER, .gap = 10.0f) {
                UI_LABEL("Component Gallery — Tree", .font = UI_FONT_BOLD,
                         .color = t->foreground);
                UI_SPACER(UI_GROW);
                if (UI_BUTTON(.id = "tree.guides", .label = "Toggle Guides",
                              .h = UI_PX(22.0f), .radius = g_config.corner_radius,
                              .fg = t->text_muted, .fg_hover = t->foreground,
                              .bg_hover = t->secondary_hover)) {
                    s_tree_guides = !s_tree_guides;
                }
                if (UI_BUTTON(.id = "tree.expand", .label = "Expand All",
                              .h = UI_PX(22.0f), .radius = g_config.corner_radius,
                              .fg = t->text_muted, .fg_hover = t->foreground,
                              .bg_hover = t->secondary_hover)) {
                    memset(s_expanded, 1, sizeof(s_expanded));
                }
                if (UI_BUTTON(.id = "tree.collapse", .label = "Collapse All",
                              .h = UI_PX(22.0f), .radius = g_config.corner_radius,
                              .fg = t->text_muted, .fg_hover = t->foreground,
                              .bg_hover = t->secondary_hover)) {
                    memset(s_expanded, 0, sizeof(s_expanded));
                }
            }

            UI_ROW(.w = UI_GROW, .h = UI_GROW, .gap = 12.0f) {
                UI_BOX(.w = UI_PX(420.0f), .h = UI_GROW, .bg = t->sidebar_bg,
                       .radius = g_config.corner_radius) {
                    ui_tree_begin(&(UiTreeProps){
                        .id = "demo.tree",
                        .row_height = 24.0f,
                        .indent = 14.0f,
                        .show_guides = s_tree_guides,
                        .selected = &s_tree_demo_sel,
                    });

                    int path[TREE_DEMO_DEPTH + 1] = {0};
                    emit_demo_subtree(path, 0);

                    UiTreeResult r = ui_tree_end();

                    static UiTreeResult s_last;
                    s_last = r;
                    if (getenv("SILVAPI_TREE_TRACE")) {
                        static int tick;
                        if (++tick % 30 == 0) {
                            fprintf(stderr, "[Tree] demo rows=%zu emitted=%zu sel=%d\n",
                                    r.row_count, r.emitted, s_tree_demo_sel);
                        }
                    }
                }

                UI_COL(.w = UI_GROW, .h = UI_GROW, .gap = 4.0f) {
                    UI_LABEL("Push-style API: the caller walks its own data; the",
                             .color = t->text_muted);
                    UI_LABEL("component owns indent, chevrons, culling, hit-testing.",
                             .color = t->text_muted);
                }
            }
        }
    }
}


// ---------------------------------------------------------------------------
// Tabs demo. The document variant is already exercised by the real request
// panel; this covers the underline variant and the reorder/overflow paths that
// the app does not reach with only a few tabs open.
// ---------------------------------------------------------------------------

static UiTab s_doc_tabs[] = {
    { .id = "t0", .label = "List Posts",   .badge = "GET",    .closable = true },
    { .id = "t1", .label = "Create Post",  .badge = "POST",   .closable = true, .dirty = true },
    { .id = "t2", .label = "Update Post",  .badge = "PATCH",  .closable = true },
    { .id = "t3", .label = "Remove Post",  .badge = "DELETE", .closable = true },
    { .id = "t4", .label = "A Very Long Request Name That Must Clamp",
      .badge = "PUT", .closable = true },
    { .id = "t5", .label = "Health",       .badge = "HEAD",   .closable = true },
    { .id = "t6", .label = "Options Probe", .badge = "OPTIONS", .closable = true },
};
static size_t s_doc_count = sizeof(s_doc_tabs) / sizeof(s_doc_tabs[0]);
static int    s_doc_active = 0;

static const UiTab SUB_TABS[] = {
    { .id = "body",    .label = "Body (Json)", .icon = ICON_FILE_CODE },
    { .id = "params",  .label = "Params",      .icon = ICON_TABLE },
    { .id = "headers", .label = "Headers (0)", .icon = ICON_LAYERS },
    { .id = "auth",    .label = "Auth",        .icon = ICON_SHIELD },
    { .id = "info",    .label = "Info",        .icon = ICON_INFO },
};
static int s_sub_active = 0;

void view_demo_tabs(Rect rect) {
    Theme* t = theme_get_active();

    UI_PANEL(rect, .bg = t->background) {
        UI_COL(.w = UI_GROW, .h = UI_GROW, .pad = 14.0f, .gap = 14.0f) {

            UI_LABEL("Component Gallery — Tabs", .font = UI_FONT_BOLD,
                     .color = t->foreground);

            UI_LABEL("Document variant: closable, reorderable, dirty marker, clamped width",
                     .color = t->text_muted);

            UiTabsResult dr = ui_tabs(&(UiTabsProps){
                .id = "demo.doctabs",
                .variant = UI_TABS_DOCUMENT,
                .tabs = s_doc_tabs,
                .tab_count = s_doc_count,
                .active = &s_doc_active,
                .height = 32.0f,
                .show_add = true,
                .show_overflow = true,
                .reorderable = true,
                .close_on_middle = true,
            });

            if (dr.closed >= 0 && s_doc_count > 1) {
                for (size_t k = (size_t)dr.closed; k + 1 < s_doc_count; k++) {
                    s_doc_tabs[k] = s_doc_tabs[k + 1];
                }
                s_doc_count--;
                if (s_doc_active >= (int)s_doc_count) s_doc_active = (int)s_doc_count - 1;
            }
            if (dr.reorder_from >= 0 && dr.reorder_to >= 0) {
                int a = dr.reorder_from, b = dr.reorder_to;
                UiTab moved = s_doc_tabs[a];
                if (a < b) for (int k = a; k < b; k++) s_doc_tabs[k] = s_doc_tabs[k + 1];
                else       for (int k = a; k > b; k--) s_doc_tabs[k] = s_doc_tabs[k - 1];
                s_doc_tabs[b] = moved;
                if (s_doc_active == a) s_doc_active = b;
            }

            UI_LABEL("Underline variant: sub-tabs, no close, sized to content",
                     .color = t->text_muted);

            ui_tabs(&(UiTabsProps){
                .id = "demo.subtabs",
                .variant = UI_TABS_UNDERLINE,
                .tabs = SUB_TABS,
                .tab_count = sizeof(SUB_TABS) / sizeof(SUB_TABS[0]),
                .active = &s_sub_active,
                .height = 30.0f,
                .min_tab_width = 1.0f,     // content-sized; no minimum
            });

            char status[192];
            snprintf(status, sizeof(status),
                     "doc active=%d count=%zu overflowing=%d   sub active=%d",
                     s_doc_active, s_doc_count, (int)dr.overflowing, s_sub_active);
            UI_LABEL(status, .color = t->text_muted, .font = UI_FONT_MONO);
        }
    }
}


// ---------------------------------------------------------------------------
// Code editor demo.
//
// A synthetic body large enough that virtualization is observable: the receipt
// reports how many of the lines were actually drawn.
// ---------------------------------------------------------------------------

#define CODE_DEMO_RECORDS 2000

static FredBuffer* s_code_buf;
static char s_code_search[64];
static bool s_code_wrap = true;

// The app owns its languages; the component only asks for spans and a palette.
static size_t json_line_tokens(const char* line, size_t len,
                               UiCodeToken* out, size_t max, void* user) {
    (void)user;
    JsonLineToken tmp[256];
    size_t n = json_tokenize_line(line, len, tmp,
                                  max < 256 ? max : 256);
    for (size_t i = 0; i < n; i++) {
        out[i].start = tmp[i].start;
        out[i].length = tmp[i].length;
        out[i].kind = tmp[i].type;
    }
    return n;
}

static void build_code_demo(void) {
    if (s_code_buf) return;

    // Built as one flat string then handed to the buffer in a single insert:
    // fredbuf_insert_string at the gap is cheap, but 14k separate inserts would
    // not be.
    size_t cap = (size_t)CODE_DEMO_RECORDS * 420 + 4096;
    char* flat = malloc(cap);
    if (!flat) return;

    size_t w = 0;
    w += (size_t)snprintf(flat + w, cap - w, "{\n  \"records\": [\n");
    for (int i = 0; i < CODE_DEMO_RECORDS && w < cap - 200; i++) {
        w += (size_t)snprintf(flat + w, cap - w,
            "    {\n"
            "      \"id\": %d,\n"
            "      \"name\": \"record-%04d\",\n"
            "      \"active\": %s,\n"
            "      \"score\": %d.%02d,\n"
            "      \"parent\": %s,\n"
            "      \"body\": \"%s\"\n"
            "    }%s\n",
            i, i, (i % 3) ? "true" : "false", i % 100, i % 97,
            (i % 7) ? "null" : "\"root\"",
            // Every fifth record carries a long value, so soft wrap has
            // something to wrap and the row count stops matching the line count.
            (i % 5 == 0)
              ? "sunt aut facere repellat provident occaecati excepturi optio "
                "reprehenderit quia et suscipit suscipit recusandae consequuntur "
                "expedita et cum reprehenderit molestiae ut ut quas totam nostrum "
                "rerum est autem sunt rem eveniet architecto"
              : "short",
            (i + 1 < CODE_DEMO_RECORDS) ? "," : "");
    }
    w += (size_t)snprintf(flat + w, cap - w, "  ]\n}\n");

    s_code_buf = fredbuf_create_from_string(flat);
    free(flat);
}

void view_demo_code(Rect rect) {
    Theme* t = theme_get_active();
    build_code_demo();
    if (!s_code_buf) return;

    // Test hook: prefill the highlight term so the search path is verifiable
    // without driving the text input.
    if (s_code_search[0] == '\0' && getenv("SILVAPI_CODE_SEARCH")) {
        snprintf(s_code_search, sizeof(s_code_search), "%s",
                 getenv("SILVAPI_CODE_SEARCH"));
    }

    // Palette indexed by JsonTokenType.
    static Color palette[8];
    palette[JSON_TOK_NONE]        = t->foreground;
    palette[JSON_TOK_KEY]         = t->info;
    palette[JSON_TOK_STRING]      = t->success;
    palette[JSON_TOK_NUMBER]      = t->warning;
    palette[JSON_TOK_BOOLEAN]     = t->primary;
    palette[JSON_TOK_NULL]        = t->danger;
    palette[JSON_TOK_PUNCTUATION] = t->text_muted;
    palette[JSON_TOK_COMMENT]     = t->text_dim;

    UI_PANEL(rect, .bg = t->background) {
        UI_COL(.w = UI_GROW, .h = UI_GROW, .pad = 10.0f, .gap = 8.0f) {

            static UiCodeEditorResult last;

            UI_ROW(.w = UI_GROW, .h = UI_PX(28.0f), .gap = 10.0f, .align_y = UI_CENTER) {
                UI_LABEL("Component Gallery — Code Editor", .font = UI_FONT_BOLD,
                         .color = t->foreground);
                UI_SPACER(UI_GROW);
                if (UI_BUTTON(.id = "code.wrap", .label = "Wrap",
                              .h = UI_PX(24.0f), .radius = g_config.corner_radius,
                              .fg = s_code_wrap ? t->primary : t->text_muted,
                              .fg_hover = t->foreground,
                              .bg_hover = t->secondary_hover)) {
                    s_code_wrap = !s_code_wrap;
                }
                UI_BOX(.w = UI_PX(220.0f), .h = UI_GROW, .align_y = UI_CENTER) {
                    UI_TEXT_INPUT(.id = "code.search",
                                  .buffer = s_code_search,
                                  .buffer_size = sizeof(s_code_search),
                                  .placeholder = "Highlight...",
                                  .h = UI_PX(24.0f),
                                  .bg = t->input_bg, .fg = t->foreground,
                                  .placeholder_color = t->input_placeholder,
                                  .border_color = t->input_border,
                                  .border_focus_color = t->input_border_focus,
                                  .caret_color = t->primary,
                                  .radius = g_config.corner_radius);
                }
                char info[160];
                snprintf(info, sizeof(info),
                         "%zu lines / %zu rows, %zu drawn   caret %zu:%zu   hits %zu   wrap %s",
                         last.line_count, last.total_rows, last.visible_rows,
                         last.caret_line + 1, last.caret_col + 1, last.search_hits,
                         s_code_wrap ? "on" : "off");
                UI_LABEL(info, .color = t->text_muted, .font = UI_FONT_MONO);
            }

            last = ui_code_editor(&(UiCodeEditorProps){
                .id = "demo.code",
                .buffer = s_code_buf,
                .show_line_numbers = true,
                .soft_wrap = s_code_wrap,
                .tokenize = json_line_tokens,
                .palette = palette,
                .palette_size = 8,
                .search = s_code_search[0] ? s_code_search : NULL,
            });
        }
    }
}


// ---------------------------------------------------------------------------
// Form controls demo. Mirrors the options panel shape: a label/description on
// the left, the control pinned right, so each control is exercised at the width
// it will actually get.
// ---------------------------------------------------------------------------

static bool  s_ctl_anim = true;
static bool  s_ctl_round = true;
static bool  s_ctl_system = true;
static bool  s_ctl_disabled_sw = false;

static float s_ctl_spacing = 6.0f;
static float s_ctl_opacity = 0.85f;
static float s_ctl_font = 15.0f;
static float s_ctl_inspector = 13.0f;

static bool  s_ctl_theme_open = false;
static int   s_ctl_theme_idx = 0;
static char  s_ctl_theme_filter[64];

static const char* const THEME_NAMES[] = { "Ayu Dark", "Ayu Light", "Catppuccin Mocha", "Gruvbox Dark" };

static const UiMenuItem THEME_ITEMS[] = {
    { .kind = UI_MENU_ACTION, .label = "Ayu Dark" },
    { .kind = UI_MENU_ACTION, .label = "Ayu Light" },
    { .kind = UI_MENU_ACTION, .label = "Catppuccin Mocha" },
    { .kind = UI_MENU_ACTION, .label = "Gruvbox Dark" },
};

// One options row: text on the left, control on the right.
static void ctl_row_begin(Theme* t, const char* label, const char* desc) {
    uil__open(0 /* row */, (UiBoxDesc){
        .w = UI_GROW, .h = UI_PX(desc ? 46.0f : 32.0f),
        .pad_x = 10.0f, .gap = 12.0f, .align_y = UI_CENTER,
    });
    UI_COL(.w = UI_GROW, .h = UI_GROW, .align_y = UI_CENTER, .gap = 0.0f) {
        UI_LABEL(label, .color = t->foreground, .ellipsize = true);
        if (desc) UI_LABEL(desc, .color = t->text_muted, .ellipsize = true);
    }
}

static void ctl_row_end(void) { uil__close(); }

void view_demo_controls(Rect rect) {
    Theme* t = theme_get_active();

    UI_PANEL(rect, .bg = t->background) {
        UI_COL(.w = UI_GROW, .h = UI_GROW, .pad = 14.0f, .gap = 4.0f) {

            UI_LABEL("Component Gallery — Form Controls", .font = UI_FONT_BOLD,
                     .color = t->foreground);

            UI_BOX(.w = UI_GROW, .h = UI_PX(1.0f), .bg = t->border) {}

            // --- switches ---
            ctl_row_begin(t, "Animations",
                          "Applies animations to popups, scrolling, and the caret.");
            UI_SWITCH(.id = "ctl.anim", .value = &s_ctl_anim);
            ctl_row_end();

            ctl_row_begin(t, "Round Corners",
                          "Applies rounding to popups, buttons, and selections.");
            UI_SWITCH(.id = "ctl.round", .value = &s_ctl_round);
            ctl_row_end();

            ctl_row_begin(t, "System Light / Dark Scheme", NULL);
            UI_SWITCH(.id = "ctl.system", .value = &s_ctl_system);
            ctl_row_end();

            ctl_row_begin(t, "Disabled switch", "Reports its state but ignores clicks.");
            UI_SWITCH(.id = "ctl.dis", .value = &s_ctl_disabled_sw, .disabled = true);
            ctl_row_end();

            // --- sliders ---
            ctl_row_begin(t, "Spacing", "Space between items in a list.");
            UI_BOX(.w = UI_PX(220.0f), .h = UI_GROW, .align_y = UI_CENTER) {
                UI_SLIDER(.id = "ctl.spacing", .value = &s_ctl_spacing,
                          .min = 0.0f, .max = 16.0f, .step = 1.0f,
                          .show_value = true, .format = "%.0f");
            }
            ctl_row_end();

            ctl_row_begin(t, "Inactive Panel Opacity", "Continuous, no step grid.");
            UI_BOX(.w = UI_PX(220.0f), .h = UI_GROW, .align_y = UI_CENTER) {
                UI_SLIDER(.id = "ctl.opacity", .value = &s_ctl_opacity,
                          .min = 0.2f, .max = 1.0f,
                          .show_value = true, .format = "%.2f");
            }
            ctl_row_end();

            // --- steppers ---
            ctl_row_begin(t, "Font Size", "Determines the scale of the whole UI.");
            UI_STEPPER(.id = "ctl.font", .value = &s_ctl_font,
                       .min = 8.0f, .max = 32.0f, .step = 1.0f,
                       .suffix = "px", .show_reset = true, .reset_value = 15.0f);
            ctl_row_end();

            ctl_row_begin(t, "Inspector Font Size", "Hold a button to repeat.");
            UI_STEPPER(.id = "ctl.insp", .value = &s_ctl_inspector,
                       .min = 8.0f, .max = 32.0f, .step = 1.0f);
            ctl_row_end();

            // --- select ---
            ctl_row_begin(t, "Dark Color Scheme",
                          "Trigger only; the caller opens the menu beside it.");
            UI_BOX(.w = UI_PX(200.0f), .h = UI_GROW, .align_y = UI_CENTER) {
                if (UI_SELECT(.id = "ctl.theme",
                              .value = THEME_NAMES[s_ctl_theme_idx],
                              .open = s_ctl_theme_open)) {
                    s_ctl_theme_open = !s_ctl_theme_open;
                }
            }
            ctl_row_end();

            if (s_ctl_theme_open) {
                Rect trigger = uil_element_rect("ctl.theme", 0);
                UI_BOX(.floating = true, .float_to_root = true,
                       .float_attach = UI_ATTACH_TOP_LEFT,
                       .float_offset = { trigger.x, trigger.y + trigger.h + 2.0f },
                       .float_z = 60) {
                    UiMenuResult mr = ui_menu(&(UiMenuProps){
                        .id = "ctl.theme.menu",
                        .items = THEME_ITEMS,
                        .item_count = sizeof(THEME_ITEMS) / sizeof(THEME_ITEMS[0]),
                        .filter_buffer = s_ctl_theme_filter,
                        .filter_buffer_size = sizeof(s_ctl_theme_filter),
                        .placeholder = "Filter themes...",
                        .width = 220.0f,
                        .selected = &s_ctl_theme_idx,
                    });
                    if (mr.activated >= 0) {
                        s_ctl_theme_idx = mr.activated;
                        s_ctl_theme_open = false;
                    }
                    if (mr.dismissed) s_ctl_theme_open = false;
                }
            }

            UI_SPACER(UI_GROW);

            char status[220];
            snprintf(status, sizeof(status),
                     "anim=%d round=%d system=%d  spacing=%.0f opacity=%.2f  "
                     "font=%.0f insp=%.0f  theme=%s",
                     s_ctl_anim, s_ctl_round, s_ctl_system,
                     (double)s_ctl_spacing, (double)s_ctl_opacity,
                     (double)s_ctl_font, (double)s_ctl_inspector,
                     THEME_NAMES[s_ctl_theme_idx]);
            UI_LABEL(status, .color = t->text_muted, .font = UI_FONT_MONO);
        }
    }
}


// ---------------------------------------------------------------------------
// Options-in-a-menu, using the trailing control slot. This replaces the mocked
// UI_MENU_VALUE rows: the switches and steppers here are the real components,
// composed into menu rows rather than reimplemented by the menu.
// ---------------------------------------------------------------------------

static bool  s_om_anim = true;
static bool  s_om_round = true;
static bool  s_om_hidden = false;
static float s_om_spacing = 6.0f;
static float s_om_font = 15.0f;
static float s_om_opacity = 0.9f;
static char  s_om_filter[64];
static int   s_om_sel = 0;

static void om_switch(const struct UiMenuItem* it, void* user) {
    (void)user;
    if (getenv("SILVAPI_OM_TRACE")) {
        Rect r = uil_element_rect(it->id, 0);
        fprintf(stderr, "[om] switch id=%s checked=%p val=%d rect=(%.0f,%.0f %.0fx%.0f)\n",
                it->id ? it->id : "(null)", (void*)it->checked,
                it->checked ? *it->checked : -1, r.x, r.y, r.w, r.h);
    }
    ui_switch((UiSwitchProps){ .id = it->id, .value = (bool*)it->checked });
}

static void om_slider(const struct UiMenuItem* it, void* user) {
    float* v = (float*)user;
    ui_slider((UiSliderProps){
        .id = it->id, .value = v,
        .min = 0.0f, .max = 16.0f, .step = 1.0f,
        .show_value = true, .format = "%.0f",
    });
}

static void om_opacity(const struct UiMenuItem* it, void* user) {
    float* v = (float*)user;
    ui_slider((UiSliderProps){
        .id = it->id, .value = v,
        .min = 0.2f, .max = 1.0f,
        .show_value = true, .format = "%.2f",
    });
}

static void om_stepper(const struct UiMenuItem* it, void* user) {
    float* v = (float*)user;
    ui_stepper((UiStepperProps){
        .id = it->id, .value = v,
        .min = 8.0f, .max = 32.0f, .step = 1.0f,
        .suffix = "px", .show_reset = true, .reset_value = 15.0f,
    });
}

static const UiCrumb DEMO_CRUMBS[] = {
    { .id = "pc",    .label = "This PC",  .icon = ICON_DATABASE },
    { .id = "os",    .label = "OS (C:)",  .icon = ICON_DATABASE },
    { .id = "users", .label = "Users",    .icon = ICON_FOLDER },
    { .id = "sures", .label = "sures",    .icon = ICON_FOLDER },
    { .id = "code",  .label = "codelang", .icon = ICON_FOLDER },
    { .id = "snake", .label = "snakepie", .icon = ICON_FOLDER_OPEN },
};

static int s_crumb_open = -1;
static int s_crumb_at = 5;

void view_demo_options(Rect rect) {
    Theme* t = theme_get_active();

    // Items are rebuilt each frame so the trailing callbacks bind to current
    // state; the menu only reads them during the call.
    static UiMenuItem items[10];
    size_t n = 0;
    items[n++] = (UiMenuItem){ .kind = UI_MENU_SECTION, .label = "Appearance",
                               .icon = ICON_SPARKLES };
    items[n++] = (UiMenuItem){ .kind = UI_MENU_CONTROL, .id = "om.anim",
                               .label = "Animations",
                               .description = "Applies animations to popups and scrolling.",
                               .checked = &s_om_anim, .trailing = om_switch,
                               .trailing_width = 30.0f };
    items[n++] = (UiMenuItem){ .kind = UI_MENU_CONTROL, .id = "om.round",
                               .label = "Round Corners",
                               .checked = &s_om_round, .trailing = om_switch,
                               .trailing_width = 30.0f };
    items[n++] = (UiMenuItem){ .kind = UI_MENU_CONTROL, .id = "om.font",
                               .label = "Font Size",
                               .description = "Determines the scale of the whole UI.",
                               .trailing = om_stepper, .user = &s_om_font,
                               .trailing_width = 140.0f };
    items[n++] = (UiMenuItem){ .kind = UI_MENU_CONTROL, .id = "om.spacing",
                               .label = "Spacing",
                               .description = "Space between items in a list.",
                               .trailing = om_slider, .user = &s_om_spacing,
                               .trailing_width = 200.0f };
    items[n++] = (UiMenuItem){ .kind = UI_MENU_CONTROL, .id = "om.opacity",
                               .label = "Inactive Panel Opacity",
                               .trailing = om_opacity, .user = &s_om_opacity,
                               .trailing_width = 200.0f };
    items[n++] = (UiMenuItem){ .kind = UI_MENU_SEPARATOR };
    items[n++] = (UiMenuItem){ .kind = UI_MENU_SECTION, .label = "Files",
                               .icon = ICON_FOLDER };
    items[n++] = (UiMenuItem){ .kind = UI_MENU_TOGGLE, .label = "Show Hidden Files",
                               .checked = &s_om_hidden, .shortcut = "Ctrl+H" };

    UI_PANEL(rect, .bg = t->background) {
        UI_COL(.w = UI_GROW, .h = UI_GROW, .pad = 14.0f, .gap = 12.0f) {

            UI_LABEL("Component Gallery — Options menu + Breadcrumb",
                     .font = UI_FONT_BOLD, .color = t->foreground);

            // --- breadcrumb ---
            UI_BOX(.w = UI_GROW, .h = UI_PX(30.0f), .bg = t->panel_bg,
                   .radius = g_config.corner_radius, .pad_x = 4.0f,
                   .align_y = UI_CENTER) {
                UiBreadcrumbResult br = ui_breadcrumb(&(UiBreadcrumbProps){
                    .id = "demo.crumbs",
                    .crumbs = DEMO_CRUMBS,
                    .crumb_count = (size_t)(s_crumb_at + 1),
                    .open_segment = s_crumb_open,
                    .show_chevrons = true,
                    .editable = true,
                });
                if (br.clicked >= 0) {
                    s_crumb_at = br.clicked;
                    s_crumb_open = -1;
                }
                if (br.chevron_clicked >= 0) {
                    s_crumb_open = (s_crumb_open == br.chevron_clicked)
                                     ? -1 : br.chevron_clicked;
                }
            }

            // --- options menu with real controls ---
            UI_ROW(.w = UI_GROW, .h = UI_GROW, .gap = 16.0f, .align_y = UI_START) {
                ui_menu(&(UiMenuProps){
                    .id = "demo.options2",
                    .items = items,
                    .item_count = n,
                    .filter_buffer = s_om_filter,
                    .filter_buffer_size = sizeof(s_om_filter),
                    .placeholder = "Select an option...",
                    .width = 620.0f,
                    .max_height = 460.0f,
                    .selected = &s_om_sel,
                });

                UI_COL(.w = UI_GROW, .gap = 4.0f) {
                    char st[220];
                    snprintf(st, sizeof(st),
                             "anim=%d round=%d hidden=%d\nspacing=%.0f font=%.0f opacity=%.2f",
                             s_om_anim, s_om_round, s_om_hidden,
                             (double)s_om_spacing, (double)s_om_font,
                             (double)s_om_opacity);
                    UI_LABEL(st, .color = t->text_muted, .font = UI_FONT_MONO);
                    char cb[128];
                    snprintf(cb, sizeof(cb), "crumb at=%d open=%d",
                             s_crumb_at, s_crumb_open);
                    UI_LABEL(cb, .color = t->text_muted, .font = UI_FONT_MONO);
                }
            }
        }
    }
}

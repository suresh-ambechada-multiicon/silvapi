#include "ui/layout.h"
#include "gfx/gfx.h"
#include "input/input.h"
#define CLAY_IMPLEMENTATION
#include "clay.h"

// ---------------------------------------------------------------------------
// Clay is confined to this file. Nothing above it sees a Clay type.
// ---------------------------------------------------------------------------

#define UIL_MAX_FONTS      8
#define UIL_TEXT_ARENA     (256 * 1024)
#define UIL_MAX_PARAGRAPHS 512

typedef struct {
    const char* text;
    size_t      len;
    UiTextDesc  desc;
    TextLayout  layout;
} ParagraphBlock;

// Icon draw payload. The tint travels with the icon rather than in the
// element's backgroundColor: Clay emits a RECTANGLE for a non-zero
// backgroundColor *in addition to* the IMAGE command, which would paint a
// solid block the same colour as the icon on top of it.
typedef struct {
    IconType icon;
    Color    tint;
} IconBlock;

static void*   s_clay_memory = NULL;
static Font*   s_fonts[UIL_MAX_FONTS];
static size_t  s_font_count = 0;
static bool    s_ready = false;

// Paragraph blocks live for one frame; Clay holds pointers to them via
// customData until the render pass consumes them.
static ParagraphBlock s_paragraphs[UIL_MAX_PARAGRAPHS];
static size_t         s_paragraph_count = 0;

static IconBlock s_icons[UIL_MAX_PARAGRAPHS];
static size_t    s_icon_count = 0;

// Custom-draw payloads. Declared here because the render pass needs the pool
// bounds to tell a paragraph block from a custom block.
#define UIL_MAX_CUSTOM      256
#define UIL_MAX_TEXT_INPUTS 128

typedef struct {
    UiDrawFn draw;
    void*    user;
} CustomBlock;

static CustomBlock s_customs[UIL_MAX_CUSTOM];
static size_t      s_custom_count = 0;
static size_t      s_text_input_count = 0;
static uint32_t    s_frame_counter = 0;

static Font* font_for(UiFontId id);

// Ellipsized label. Drawn through uil_custom so the text engine performs the
// truncation against the box the solver actually produced -- the natural width
// is not known to be too large until then.
typedef struct {
    const char* text;
    UiTextDesc  desc;
} EllipsisBlock;

#define UIL_MAX_ELLIPSIS 256
static EllipsisBlock s_ellipsis[UIL_MAX_ELLIPSIS];
static size_t        s_ellipsis_count = 0;

static void draw_ellipsized(Rect r, void* user) {
    EllipsisBlock* b = (EllipsisBlock*)user;
    if (!b) return;
    Font* font = font_for(b->desc.font);
    font_draw_text_ellipsized(font, b->text, 0, r,
                              b->desc.align, TEXT_VALIGN_MIDDLE, b->desc.color);
}

static bool is_paragraph_block(const void* p) {
    return p >= (const void*)&s_paragraphs[0] &&
           p <  (const void*)&s_paragraphs[UIL_MAX_PARAGRAPHS];
}

// Per-frame copies of caller strings. Clay keeps string pointers alive until
// Clay_EndLayout, so a caller passing a stack buffer would otherwise dangle.
static char*   s_text_arena = NULL;
static size_t  s_text_head = 0;

static Font* font_for(UiFontId id) {
    size_t i = (size_t)id;
    if (i >= s_font_count || !s_fonts[i]) return s_fonts[0];
    return s_fonts[i];
}

static Clay_Color to_clay_color(Color c) {
    return (Clay_Color){ c.r * 255.0f, c.g * 255.0f, c.b * 255.0f, c.a * 255.0f };
}

static Color from_clay_color(Clay_Color c) {
    return (Color){ c.r / 255.0f, c.g / 255.0f, c.b / 255.0f, c.a / 255.0f };
}

static Rect from_clay_bbox(Clay_BoundingBox b) {
    return rect_make(b.x, b.y, b.width, b.height);
}

// Copies `text` into the frame arena and returns a Clay slice over the copy.
static Clay_String frame_string(const char* text, size_t len) {
    Clay_String out = {0};
    if (!text || len == 0 || !s_text_arena) return out;
    if (s_text_head + len + 1 > UIL_TEXT_ARENA) return out;  // arena exhausted

    char* dst = s_text_arena + s_text_head;
    memcpy(dst, text, len);
    dst[len] = '\0';
    s_text_head += len + 1;

    out.chars = dst;
    out.length = (int32_t)len;
    out.isStaticallyAllocated = false;
    return out;
}

// ---------------------------------------------------------------------------
// Text measurement. Routed through the shaped-run cache, so a width reported
// to the solver is byte-for-byte the width that will be drawn.
// ---------------------------------------------------------------------------

static Clay_Dimensions measure_text(Clay_StringSlice text, Clay_TextElementConfig* config, void* user) {
    (void)user;
    Font* font = font_for(config ? (UiFontId)config->fontId : UI_FONT_UI);
    if (!font || !text.chars || text.length <= 0) {
        return (Clay_Dimensions){ 0.0f, font ? font_get_line_height(font) : 0.0f };
    }

    float w = font_measure_text(font, text.chars, (size_t)text.length);
    float h = (config && config->lineHeight > 0) ? (float)config->lineHeight
                                                 : font_get_line_height(font);
    return (Clay_Dimensions){ w, h };
}

static void on_clay_error(Clay_ErrorData err) {
    fprintf(stderr, "[Layout] Clay: %.*s\n", (int)err.errorText.length, err.errorText.chars);
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

bool uil_layout_init(Font* const* fonts, size_t font_count) {
    if (s_ready) return true;
    if (!fonts || font_count == 0) return false;

    s_font_count = font_count < UIL_MAX_FONTS ? font_count : UIL_MAX_FONTS;
    for (size_t i = 0; i < s_font_count; i++) s_fonts[i] = fonts[i];

    size_t need = Clay_MinMemorySize();
    s_clay_memory = malloc(need);
    if (!s_clay_memory) return false;

    s_text_arena = malloc(UIL_TEXT_ARENA);
    if (!s_text_arena) {
        free(s_clay_memory);
        s_clay_memory = NULL;
        return false;
    }

    Clay_Arena arena = Clay_CreateArenaWithCapacityAndMemory(need, s_clay_memory);
    Clay_Initialize(arena, (Clay_Dimensions){ 0, 0 },
                    (Clay_ErrorHandler){ .errorHandlerFunction = on_clay_error });
    Clay_SetMeasureTextFunction(measure_text, NULL);

    s_ready = true;
    return true;
}

void uil_layout_shutdown(void) {
    free(s_clay_memory);
    s_clay_memory = NULL;
    free(s_text_arena);
    s_text_arena = NULL;
    s_font_count = 0;
    s_ready = false;
}

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

static Clay_ElementId element_id(const char* id, int32_t index) {
    if (!id) return (Clay_ElementId){0};
    Clay_String s = { .isStaticallyAllocated = false,
                      .length = (int32_t)strlen(id),
                      .chars = id };
    return Clay_GetElementIdWithIndex(s, (uint32_t)index);
}

// ---------------------------------------------------------------------------
// Frame
// ---------------------------------------------------------------------------

static Rect s_root = {0};
static bool   s_overlap_check = false;
static bool   s_debug_text_boxes = false;
static bool   s_trace_buttons = false;
static size_t s_overlap_count = 0;

void uil_set_overlap_check(bool enabled) { s_overlap_check = enabled; }
void uil_set_debug_text_boxes(bool enabled) { s_debug_text_boxes = enabled; }
void uil_set_trace_buttons(bool enabled)     { s_trace_buttons = enabled; }
bool uil_overlap_check_enabled(void)     { return s_overlap_check; }
size_t uil_overlap_count(void)           { return s_overlap_count; }

// Text boxes that merely touch are fine; only a real intersection is a bug.
static bool rects_overlap(Rect a, Rect b) {
    const float eps = 0.5f;
    return (a.x + a.w - eps > b.x) && (b.x + b.w - eps > a.x) &&
           (a.y + a.h - eps > b.y) && (b.y + b.h - eps > a.y);
}

typedef struct {
    Rect        box;
    const char* chars;
    int32_t     len;
    int16_t     z;
} TextBox;

#define UIL_MAX_TEXT_BOXES 1024

static void check_text_overlap(Clay_RenderCommandArray commands, float ox, float oy) {
    static TextBox boxes[UIL_MAX_TEXT_BOXES];
    size_t n = 0;

    // Compare what is actually VISIBLE, not the raw layout box. A virtualized
    // list deliberately lays out rows past the edge of its viewport and relies
    // on the scissor to cut them; comparing unclipped boxes reports those as
    // overlaps when nothing overlaps on screen.
    Rect clip_stack[32];
    int  clip_depth = 0;

    for (int32_t i = 0; i < commands.length && n < UIL_MAX_TEXT_BOXES; i++) {
        Clay_RenderCommand* cmd = Clay_RenderCommandArray_Get(&commands, i);
        if (!cmd) continue;

        if (cmd->commandType == CLAY_RENDER_COMMAND_TYPE_SCISSOR_START) {
            Rect r = from_clay_bbox(cmd->boundingBox);
            r.x += ox;
            r.y += oy;
            if (clip_depth < 32) {
                clip_stack[clip_depth] = (clip_depth > 0)
                                            ? rect_intersect(clip_stack[clip_depth - 1], r)
                                            : r;
                clip_depth++;
            }
            continue;
        }
        if (cmd->commandType == CLAY_RENDER_COMMAND_TYPE_SCISSOR_END) {
            if (clip_depth > 0) clip_depth--;
            continue;
        }
        if (cmd->commandType != CLAY_RENDER_COMMAND_TYPE_TEXT) continue;

        Clay_TextRenderData* d = &cmd->renderData.text;
        if (d->stringContents.length <= 0) continue;

        Rect box = from_clay_bbox(cmd->boundingBox);
        box.x += ox;
        box.y += oy;
        if (clip_depth > 0) box = rect_intersect(clip_stack[clip_depth - 1], box);
        // Entirely clipped away: not on screen, so it cannot overlap anything.
        if (box.w <= 0.5f || box.h <= 0.5f) continue;

        boxes[n].box = box;
        boxes[n].chars = d->stringContents.chars;
        boxes[n].len = d->stringContents.length;
        boxes[n].z = cmd->zIndex;
        n++;
    }

    s_overlap_count = 0;
    for (size_t i = 0; i < n; i++) {
        for (size_t k = i + 1; k < n; k++) {
            // Different z-indexes are layered on purpose (menus over content).
            if (boxes[i].z != boxes[k].z) continue;
            if (!rects_overlap(boxes[i].box, boxes[k].box)) continue;

            s_overlap_count++;
            if (s_overlap_count <= 8) {
                fprintf(stderr,
                        "[Layout] TEXT OVERLAP: \"%.*s\" (%.1f,%.1f %.1fx%.1f) "
                        "vs \"%.*s\" (%.1f,%.1f %.1fx%.1f)\n",
                        boxes[i].len, boxes[i].chars,
                        boxes[i].box.x, boxes[i].box.y, boxes[i].box.w, boxes[i].box.h,
                        boxes[k].len, boxes[k].chars,
                        boxes[k].box.x, boxes[k].box.y, boxes[k].box.w, boxes[k].box.h);
            }
        }
    }
}

static UiLayoutInput s_input = {0};

void uil_layout_begin(Rect root, UiLayoutInput input) {
    if (!s_ready) return;

    s_input = input;
    s_root = root;
    s_text_head = 0;
    s_paragraph_count = 0;
    s_icon_count = 0;
    s_custom_count = 0;
    s_text_input_count = 0;
    s_ellipsis_count = 0;
    s_frame_counter++;

    Clay_SetLayoutDimensions((Clay_Dimensions){ root.w, root.h });
    // Clay works in layout-local space, so the pointer is rebased onto `root`.
    Clay_SetPointerState((Clay_Vector2){ input.mouse.x - root.x, input.mouse.y - root.y },
                         input.mouse_down[0]);
    Clay_UpdateScrollContainers(true,
                                (Clay_Vector2){ input.scroll_delta.x, input.scroll_delta.y },
                                input.dt);
    Clay_BeginLayout();

    // Root element: an absolutely-positioned container matching `root`, so the
    // caller can hand any sub-rect of the window to the layout.
    Clay__OpenElement();
    Clay__ConfigureOpenElement((Clay_ElementDeclaration){
        .layout = {
            .sizing = { .width  = CLAY_SIZING_FIXED(root.w),
                        .height = CLAY_SIZING_FIXED(root.h) },
            .layoutDirection = CLAY_TOP_TO_BOTTOM,
        },
    });
}

static void render_commands(Clay_RenderCommandArray commands, float origin_x, float origin_y) {

    for (int32_t i = 0; i < commands.length; i++) {
        Clay_RenderCommand* cmd = Clay_RenderCommandArray_Get(&commands, i);
        if (!cmd) continue;

        Rect r = from_clay_bbox(cmd->boundingBox);
        r.x += origin_x;
        r.y += origin_y;

        switch (cmd->commandType) {
            case CLAY_RENDER_COMMAND_TYPE_RECTANGLE: {
                Clay_RectangleRenderData* d = &cmd->renderData.rectangle;
                Color col = from_clay_color(d->backgroundColor);
                if (col.a <= 0.0f) break;
                float radius = d->cornerRadius.topLeft;
                if (radius > 0.0f) r_draw_rect_rounded(r, radius, col);
                else               r_draw_rect(r, col);
                break;
            }

            case CLAY_RENDER_COMMAND_TYPE_BORDER: {
                Clay_BorderRenderData* d = &cmd->renderData.border;
                float w = (float)d->width.top;
                if (w <= 0.0f) w = 1.0f;
                // Clay emits BORDER after an element's children, so this must
                // stroke only -- filling here would cover the content.
                r_draw_rect_rounded_outline(r, d->cornerRadius.topLeft, w,
                                            from_clay_color(d->color));
                break;
            }

            case CLAY_RENDER_COMMAND_TYPE_TEXT: {
                Clay_TextRenderData* d = &cmd->renderData.text;
                Font* font = font_for((UiFontId)d->fontId);
                // SILVAPI_TEXT_BOXES tints every text box, which makes a
                // suspected overlap or clipping problem visible at a glance.
                if (s_debug_text_boxes) {
                    r_draw_rect(r, color_rgba(1.0f, 0.0f, 0.0f, 0.30f));
                }
                // Clay reports the line's bounding box; the text engine expects
                // the top of an em box, and centres capitals within it.
                font_draw_text_aligned(font, d->stringContents.chars,
                                       (size_t)d->stringContents.length, r,
                                       TEXT_ALIGN_LEFT, TEXT_VALIGN_MIDDLE,
                                       from_clay_color(d->textColor));
                break;
            }

            case CLAY_RENDER_COMMAND_TYPE_CUSTOM: {
                Clay_CustomRenderData* d = &cmd->renderData.custom;
                if (!d->customData) break;
                // Two kinds of custom payload share this command; they are told
                // apart by which per-frame pool the pointer came from.
                if (is_paragraph_block(d->customData)) {
                    ParagraphBlock* p = (ParagraphBlock*)d->customData;
                    Font* font = font_for(p->desc.font);
                    font_draw_text_wrapped_aligned(font, &p->layout, p->text, r,
                                                   p->desc.align, p->desc.color);
                } else {
                    CustomBlock* cb = (CustomBlock*)d->customData;
                    if (cb->draw) cb->draw(r, cb->user);
                }
                break;
            }

            case CLAY_RENDER_COMMAND_TYPE_IMAGE: {
                Clay_ImageRenderData* d = &cmd->renderData.image;
                if (!d->imageData) break;
                const IconBlock* ib = (const IconBlock*)d->imageData;
                icon_draw(ib->icon, r, ib->tint);
                break;
            }

            case CLAY_RENDER_COMMAND_TYPE_SCISSOR_START:
                r_push_clip(r);
                break;

            case CLAY_RENDER_COMMAND_TYPE_SCISSOR_END:
                r_pop_clip();
                break;

            default:
                break;
        }
    }
}

void uil_layout_end(void) {
    if (!s_ready) return;

    Clay__CloseElement();  // root
    Clay_RenderCommandArray commands = Clay_EndLayout(0.0f);

    if (getenv("SILVAPI_LAYOUT_TRACE")) {
        int counts[16] = {0};
        for (int32_t i = 0; i < commands.length; i++) {
            Clay_RenderCommand* c = Clay_RenderCommandArray_Get(&commands, i);
            if (c && c->commandType < 16) counts[c->commandType]++;
        }
        fprintf(stderr, "[Layout] %d cmds: rect=%d text=%d border=%d image=%d custom=%d clip=%d/%d\n",
                commands.length, counts[CLAY_RENDER_COMMAND_TYPE_RECTANGLE],
                counts[CLAY_RENDER_COMMAND_TYPE_TEXT], counts[CLAY_RENDER_COMMAND_TYPE_BORDER],
                counts[CLAY_RENDER_COMMAND_TYPE_IMAGE], counts[CLAY_RENDER_COMMAND_TYPE_CUSTOM],
                counts[CLAY_RENDER_COMMAND_TYPE_SCISSOR_START],
                counts[CLAY_RENDER_COMMAND_TYPE_SCISSOR_END]);
    }

    if (s_overlap_check) check_text_overlap(commands, s_root.x, s_root.y);
    render_commands(commands, s_root.x, s_root.y);

    // Paragraph line arrays were heap-allocated during declaration.
    for (size_t i = 0; i < s_paragraph_count; i++) {
        font_layout_free(&s_paragraphs[i].layout);
    }
    s_paragraph_count = 0;
}

// ---------------------------------------------------------------------------
// Element scopes
// ---------------------------------------------------------------------------

static Clay_SizingAxis to_clay_sizing(UiSize s) {
    switch (s.kind) {
        case UI_SIZE_FIXED:   return CLAY_SIZING_FIXED(s.value);
        case UI_SIZE_PERCENT: return CLAY_SIZING_PERCENT(s.value);
        case UI_SIZE_GROW:
            return s.min > 0.0f ? CLAY_SIZING_GROW(.min = s.min) : CLAY_SIZING_GROW(0);
        case UI_SIZE_FIT:
        default:
            return CLAY_SIZING_FIT(0);
    }
}

static Clay_LayoutAlignmentX to_clay_align_x(UiAlign a) {
    switch (a) {
        case UI_CENTER: return CLAY_ALIGN_X_CENTER;
        case UI_END:    return CLAY_ALIGN_X_RIGHT;
        default:        return CLAY_ALIGN_X_LEFT;
    }
}

static Clay_LayoutAlignmentY to_clay_align_y(UiAlign a) {
    switch (a) {
        case UI_CENTER: return CLAY_ALIGN_Y_CENTER;
        case UI_END:    return CLAY_ALIGN_Y_BOTTOM;
        default:        return CLAY_ALIGN_Y_TOP;
    }
}

void uil__open(int direction, UiBoxDesc d) {
    if (!s_ready) return;

    // Padding resolution: `pad` is the base, axis overrides beat it, and
    // per-side overrides beat those.
    float l = d.pad, r = d.pad, t = d.pad, b = d.pad;
    if (d.pad_x != 0.0f) { l = r = d.pad_x; }
    if (d.pad_y != 0.0f) { t = b = d.pad_y; }
    if (d.pad_l != 0.0f) l = d.pad_l;
    if (d.pad_r != 0.0f) r = d.pad_r;
    if (d.pad_t != 0.0f) t = d.pad_t;
    if (d.pad_b != 0.0f) b = d.pad_b;

    Clay_ElementDeclaration decl = {
        .layout = {
            .sizing = { .width = to_clay_sizing(d.w), .height = to_clay_sizing(d.h) },
            .padding = { (uint16_t)l, (uint16_t)r, (uint16_t)t, (uint16_t)b },
            .childGap = (uint16_t)d.gap,
            .childAlignment = { to_clay_align_x(d.align_x), to_clay_align_y(d.align_y) },
            .layoutDirection = direction ? CLAY_TOP_TO_BOTTOM : CLAY_LEFT_TO_RIGHT,
        },
        .backgroundColor = to_clay_color(d.bg),
        .cornerRadius = { d.radius, d.radius, d.radius, d.radius },
    };

    if (d.border > 0.0f) {
        uint16_t bw = (uint16_t)d.border;
        decl.border.width = (Clay_BorderWidth){ bw, bw, bw, bw, 0 };
        decl.border.color = to_clay_color(d.border_color);
    }

    if (d.clip_x || d.clip_y || d.scroll_x || d.scroll_y) {
        decl.clip.horizontal = d.clip_x || d.scroll_x;
        decl.clip.vertical   = d.clip_y || d.scroll_y;
        // Clay owns the scroll offset for this element, keyed by its id.
        if (d.scroll_x || d.scroll_y) {
            decl.clip.childOffset = Clay_GetScrollOffset();
        }
    }

    if (d.floating) {
        static const Clay_FloatingAttachPointType map[] = {
            CLAY_ATTACH_POINT_LEFT_TOP,     CLAY_ATTACH_POINT_CENTER_TOP,
            CLAY_ATTACH_POINT_RIGHT_TOP,    CLAY_ATTACH_POINT_LEFT_CENTER,
            CLAY_ATTACH_POINT_CENTER_CENTER,CLAY_ATTACH_POINT_RIGHT_CENTER,
            CLAY_ATTACH_POINT_LEFT_BOTTOM,  CLAY_ATTACH_POINT_CENTER_BOTTOM,
            CLAY_ATTACH_POINT_RIGHT_BOTTOM,
        };
        int a = (int)d.float_attach;
        if (a < 0 || a >= (int)(sizeof(map) / sizeof(map[0]))) a = 0;

        decl.floating.attachTo = d.float_to_root ? CLAY_ATTACH_TO_ROOT : CLAY_ATTACH_TO_PARENT;
        decl.floating.attachPoints = (Clay_FloatingAttachPoints){ map[a], map[a] };
        decl.floating.offset = (Clay_Vector2){ d.float_offset.x, d.float_offset.y };
        decl.floating.zIndex = (int16_t)d.float_z;
        if (d.float_clip_to_parent) {
            decl.floating.clipTo = CLAY_CLIP_TO_ATTACHED_PARENT;
        }
    }

    if (d.id) {
        Clay__OpenElementWithId(element_id(d.id, d.index));
    } else {
        Clay__OpenElement();
    }
    Clay__ConfigureOpenElement(decl);
}

void uil__close(void) {
    if (!s_ready) return;
    Clay__CloseElement();
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

bool uil_hovered(void) {
    return s_ready ? Clay_Hovered() : false;
}

bool uil_element_hovered(const char* id, int32_t index) {
    if (!s_ready || !id) return false;
    return Clay_PointerOver(element_id(id, index));
}

Rect uil_element_rect(const char* id, int32_t index) {
    if (!s_ready || !id) return rect_make(0, 0, 0, 0);
    Clay_ElementData data = Clay_GetElementData(element_id(id, index));
    if (!data.found) return rect_make(0, 0, 0, 0);
    Rect r = from_clay_bbox(data.boundingBox);
    r.x += s_root.x;
    r.y += s_root.y;
    return r;
}

Vec2 uil_scroll_offset(const char* id, int32_t index) {
    if (!s_ready || !id) return (Vec2){0, 0};
    Clay_ScrollContainerData d = Clay_GetScrollContainerData(element_id(id, index));
    if (!d.found) return (Vec2){0, 0};
    return (Vec2){ d.scrollPosition->x, d.scrollPosition->y };
}

// ---------------------------------------------------------------------------
// Leaves
// ---------------------------------------------------------------------------

void uil_label(const char* text, UiTextDesc desc) {
    if (!s_ready || !text || !*text) return;
    Font* font = font_for(desc.font);

    Clay_String s = frame_string(text, strlen(text));
    if (!s.chars) return;

    if (desc.ellipsize && s_ellipsis_count < UIL_MAX_ELLIPSIS) {
        EllipsisBlock* b = &s_ellipsis[s_ellipsis_count++];
        b->text = s.chars;   // frame-arena copy, alive until end of layout
        b->desc = desc;
        // Grows to the space available; the engine trims to fit at draw time.
        uil_custom(UI_GROW, UI_PX(font_get_line_height(font)), draw_ellipsized, b);
        return;
    }

    Clay__OpenTextElement(s, (Clay_TextElementConfig){
        .textColor = to_clay_color(desc.color),
        .fontId = (uint16_t)desc.font,
        .fontSize = (uint16_t)font_get_height(font),
        .lineHeight = (uint16_t)(desc.line_height > 0.0f ? desc.line_height
                                                        : font_get_line_height(font)),
        // Wrapping is the text engine's job; a label is one line.
        .wrapMode = CLAY_TEXT_WRAP_NONE,
    });
}

void uil_paragraph(const char* text, UiTextDesc desc) {
    if (!s_ready || !text || !*text) return;
    if (s_paragraph_count >= UIL_MAX_PARAGRAPHS) return;

    Font* font = font_for(desc.font);
    size_t len = strlen(text);

    Clay_String owned = frame_string(text, len);
    if (!owned.chars) return;

    ParagraphBlock* block = &s_paragraphs[s_paragraph_count++];
    block->text = owned.chars;
    block->len = len;
    block->desc = desc;

    // The available width is not known until the solver runs, so the paragraph
    // is measured against the current layout width. Clay then treats it as a
    // fixed block. This is the deliberate trade for correct line breaking:
    // the text engine decides where lines break, not Clay.
    float avail = s_root.w > 0.0f ? s_root.w : 400.0f;
    block->layout = font_layout_text_wrapped(font, block->text, len, avail,
                                             desc.line_height, NULL);

    Clay__OpenElement();
    Clay__ConfigureOpenElement((Clay_ElementDeclaration){
        .layout = {
            .sizing = { .width  = CLAY_SIZING_GROW(0),
                        .height = CLAY_SIZING_FIXED(block->layout.total_height) },
        },
        .custom = { .customData = block },
    });
    Clay__CloseElement();
}

void uil_icon(IconType icon, float size, Color color) {
    if (!s_ready || icon == ICON_NONE) return;
    if (s_icon_count >= UIL_MAX_PARAGRAPHS) return;

    IconBlock* block = &s_icons[s_icon_count++];
    block->icon = icon;
    block->tint = color.a > 0.0f ? color : (Color){1, 1, 1, 1};

    Clay__OpenElement();
    Clay__ConfigureOpenElement((Clay_ElementDeclaration){
        .layout = {
            .sizing = { .width  = CLAY_SIZING_FIXED(size),
                        .height = CLAY_SIZING_FIXED(size) },
        },
        .image = { .imageData = block },
    });
    Clay__CloseElement();
}

void uil_spacer(UiSize size) {
    if (!s_ready) return;
    Clay__OpenElement();
    Clay__ConfigureOpenElement((Clay_ElementDeclaration){
        .layout = { .sizing = { .width = to_clay_sizing(size),
                                .height = to_clay_sizing(size) } },
    });
    Clay__CloseElement();
}

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

bool uil_button(UiButtonDesc d) {
    if (!s_ready) return false;

    Font* font = font_for(UI_FONT_UI);
    float icon_size = d.icon_size > 0.0f ? d.icon_size : font_get_height(font);
    float pad_x = d.pad_x > 0.0f ? d.pad_x : 8.0f;
    float gap   = d.gap   > 0.0f ? d.gap   : 6.0f;

    bool hovered = false;
    bool clicked = false;

    UiBoxDesc box = {
        .id = d.id, .index = d.index,
        .w = d.w.kind ? d.w : UI_FIT,
        .h = d.h.kind ? d.h : UI_GROW,
        .pad_x = pad_x,
        .gap = gap,
        .align_x = UI_CENTER,
        .align_y = UI_CENTER,
        .radius = d.radius,
    };

    // Hover is resolved before the background colour is chosen, so the element
    // is opened twice-free: uil__open needs the final colour up front.
    if (d.id) hovered = uil_element_hovered(d.id, d.index);
    box.bg = hovered && d.bg_hover.a > 0.0f ? d.bg_hover : d.bg;

    Color fg = hovered && d.fg_hover.a > 0.0f ? d.fg_hover : d.fg;

    uil__open(0 /* row */, box);
    if (d.icon != ICON_NONE) uil_icon(d.icon, icon_size, fg);
    if (d.label && *d.label) uil_label(d.label, (UiTextDesc){ .color = fg });
    clicked = hovered && s_input.mouse_clicked[0];
    uil__close();

    if (s_trace_buttons && d.id) {
        Rect br = uil_element_rect(d.id, d.index);
        fprintf(stderr, "[Button] %s[%d] rect=(%.0f,%.0f %.0fx%.0f) hover=%d click=%d\n",
                d.id, d.index, br.x, br.y, br.w, br.h, hovered, clicked);
    }

    return clicked;
}

// ===========================================================================
// Retained widget state
//
// Focus, caret, and scroll survive across frames without the caller holding
// anything: state is keyed by the widget's id hash. This is what lets a
// declarative call site stay a single expression.
// ===========================================================================

#define UIL_MAX_WIDGET_STATE 256

typedef struct {
    uint64_t key;
    size_t   caret;
    size_t   sel_anchor;
    float    scroll_x;
    uint32_t last_frame;
    bool     used;
} WidgetState;

static WidgetState s_widgets[UIL_MAX_WIDGET_STATE];
static uint64_t    s_focus_key = 0;

static uint64_t widget_key(const char* id, int32_t index) {
    if (!id) return 0;
    uint64_t h = 1469598103934665603ULL;
    for (const char* p = id; *p; p++) {
        h ^= (uint8_t)*p;
        h *= 1099511628211ULL;
    }
    h ^= (uint64_t)(uint32_t)index * 1099511628211ULL;
    return h ? h : 1;
}

static WidgetState* widget_state(uint64_t key) {
    if (!key) return NULL;
    size_t base = (size_t)(key % UIL_MAX_WIDGET_STATE);

    WidgetState* victim = NULL;
    for (int probe = 0; probe < 16; probe++) {
        WidgetState* w = &s_widgets[(base + (size_t)probe) % UIL_MAX_WIDGET_STATE];
        if (w->used && w->key == key) {
            w->last_frame = s_frame_counter;
            return w;
        }
        if (!w->used) { victim = w; break; }
        // Reclaim the least recently touched slot in the probe window.
        if (!victim || w->last_frame < victim->last_frame) victim = w;
    }
    if (!victim) return NULL;

    memset(victim, 0, sizeof(*victim));
    victim->key = key;
    victim->used = true;
    victim->last_frame = s_frame_counter;
    return victim;
}

Font* uil_font(UiFontId id) { return font_for(id); }

uint64_t uil_focused(void)   { return s_focus_key; }
void     uil_clear_focus(void) { s_focus_key = 0; }
void     uil_set_focus(uint64_t key) { s_focus_key = key; }

// ===========================================================================
// Custom draw blocks
// ===========================================================================

void uil_custom(UiSize w, UiSize h, UiDrawFn draw, void* user_data) {
    if (!s_ready || !draw) return;
    if (s_custom_count >= UIL_MAX_CUSTOM) return;

    CustomBlock* b = &s_customs[s_custom_count++];
    b->draw = draw;
    b->user = user_data;

    Clay__OpenElement();
    Clay__ConfigureOpenElement((Clay_ElementDeclaration){
        .layout = { .sizing = { .width = to_clay_sizing(w), .height = to_clay_sizing(h) } },
        .custom = { .customData = b },
    });
    Clay__CloseElement();
}

// ===========================================================================
// Modal
// ===========================================================================

void uil_modal_begin(UiModalDesc d) {
    if (!s_ready) return;

    int z = d.z > 0 ? d.z : 100;

    // Full-window scrim, so clicks outside the panel do not reach the content
    // behind it.
    if (d.scrim.a > 0.0f) {
        Clay__OpenElement();
        Clay__ConfigureOpenElement((Clay_ElementDeclaration){
            .layout = { .sizing = { .width  = CLAY_SIZING_FIXED(s_root.w),
                                    .height = CLAY_SIZING_FIXED(s_root.h) } },
            .backgroundColor = to_clay_color(d.scrim),
            .floating = {
                .attachTo = CLAY_ATTACH_TO_ROOT,
                .zIndex = (int16_t)z,
                .attachPoints = { CLAY_ATTACH_POINT_LEFT_TOP, CLAY_ATTACH_POINT_LEFT_TOP },
            },
        });
        Clay__CloseElement();
    }

    Clay_ElementDeclaration decl = {
        .layout = {
            .sizing = { .width  = d.w > 0.0f ? CLAY_SIZING_FIXED(d.w) : CLAY_SIZING_FIT(0),
                        .height = d.h > 0.0f ? CLAY_SIZING_FIXED(d.h) : CLAY_SIZING_FIT(0) },
            .padding = { (uint16_t)d.pad, (uint16_t)d.pad, (uint16_t)d.pad, (uint16_t)d.pad },
            .layoutDirection = CLAY_TOP_TO_BOTTOM,
        },
        .backgroundColor = to_clay_color(d.bg),
        .cornerRadius = { d.radius, d.radius, d.radius, d.radius },
        .floating = {
            .attachTo = CLAY_ATTACH_TO_ROOT,
            .zIndex = (int16_t)(z + 1),
            // Centre the panel's middle on the root's middle.
            .attachPoints = { CLAY_ATTACH_POINT_CENTER_CENTER, CLAY_ATTACH_POINT_CENTER_CENTER },
        },
    };
    if (d.border > 0.0f) {
        uint16_t bw = (uint16_t)d.border;
        decl.border.width = (Clay_BorderWidth){ bw, bw, bw, bw, 0 };
        decl.border.color = to_clay_color(d.border_color);
    }

    if (d.id) Clay__OpenElementWithId(element_id(d.id, 0));
    else      Clay__OpenElement();
    Clay__ConfigureOpenElement(decl);
}

void uil_modal_end(void) {
    if (!s_ready) return;
    Clay__CloseElement();
}

// ===========================================================================
// Checkbox
// ===========================================================================

bool uil_checkbox(UiCheckboxDesc d) {
    if (!s_ready || !d.value) return false;

    float size = d.size > 0.0f ? d.size : 16.0f;
    bool hovered = d.id ? uil_element_hovered(d.id, d.index) : false;
    bool toggled = hovered && s_input.mouse_clicked[0];
    if (toggled) *d.value = !*d.value;

    UiBoxDesc box = {
        .id = d.id, .index = d.index,
        .w = UI_PX(size), .h = UI_PX(size),
        .radius = d.radius > 0.0f ? d.radius : 3.0f,
        .bg = *d.value ? d.check_color : d.bg,
        .border = 1.0f,
        .border_color = d.border_color,
        .align_x = UI_CENTER, .align_y = UI_CENTER,
    };

    uil__open(0, box);
    if (*d.value) {
        // The tick is inset so it reads as inside the box at small sizes.
        uil_icon(ICON_CHECK, size - 4.0f, d.bg.a > 0.0f ? d.bg : (Color){1, 1, 1, 1});
    }
    uil__close();

    return toggled;
}

// ===========================================================================
// Text input
// ===========================================================================

typedef struct {
    UiTextInputDesc desc;
    uint64_t        key;
    bool            focused;
} TextInputBlock;


static TextInputBlock s_text_inputs[UIL_MAX_TEXT_INPUTS];

// Draws the value, placeholder, and caret inside the input's solved rect. Text
// is clipped to the box and horizontally scrolled to keep the caret visible, so
// a long value can never spill past the field.
static void draw_text_input(Rect r, void* user) {
    TextInputBlock* b = (TextInputBlock*)user;
    if (!b) return;

    const UiTextInputDesc* d = &b->desc;
    Font* font = font_for(d->font);
    WidgetState* st = widget_state(b->key);

    const char* text = d->buffer ? d->buffer : "";
    size_t len = strlen(text);

    // Password fields render as bullets but measure the same way.
    char masked[256];
    if (d->password && len > 0) {
        size_t n = len < sizeof(masked) - 1 ? len : sizeof(masked) - 1;
        memset(masked, '*', n);
        masked[n] = '\0';
        text = masked;
        len = n;
    }

    r_push_clip(r);

    if (len == 0 && d->placeholder && *d->placeholder) {
        font_draw_text_aligned(font, d->placeholder, 0, r,
                               TEXT_ALIGN_LEFT, TEXT_VALIGN_MIDDLE, d->placeholder_color);
    } else if (len > 0) {
        float scroll = st ? st->scroll_x : 0.0f;
        Rect tr = r;
        tr.x -= scroll;
        // Widen so the clip, not the box, decides what is visible.
        tr.w = font_measure_text(font, text, len) + 2.0f;
        font_draw_text_aligned(font, text, len, tr,
                               TEXT_ALIGN_LEFT, TEXT_VALIGN_MIDDLE, d->fg);
    }

    if (b->focused && st) {
        size_t caret = st->caret > len ? len : st->caret;
        float cx = caret > 0 ? font_measure_text(font, text, caret) : 0.0f;
        float scroll = st->scroll_x;

        FontMetrics m = font_metrics(font);
        float ch = m.cap_height + m.descent + 2.0f;
        float cy = r.y + (r.h - ch) * 0.5f;

        Rect caret_r = rect_make(floorf(r.x + cx - scroll), cy, 1.0f, ch);
        r_draw_rect(caret_r, d->caret_color.a > 0.0f ? d->caret_color : d->fg);
    }

    r_pop_clip();
}

// Keeps the caret inside the visible box by adjusting the horizontal scroll.
static void update_input_scroll(TextInputBlock* b, float box_w) {
    WidgetState* st = widget_state(b->key);
    if (!st || !b->desc.buffer) return;

    Font* font = font_for(b->desc.font);
    size_t len = strlen(b->desc.buffer);
    size_t caret = st->caret > len ? len : st->caret;

    float cx = caret > 0 ? font_measure_text(font, b->desc.buffer, caret) : 0.0f;
    float inner = box_w - 4.0f;
    if (inner < 1.0f) return;

    if (cx - st->scroll_x > inner)  st->scroll_x = cx - inner;
    if (cx - st->scroll_x < 0.0f)   st->scroll_x = cx;
    if (st->scroll_x < 0.0f)        st->scroll_x = 0.0f;
}

bool uil_text_input(UiTextInputDesc d) {
    if (!s_ready || !d.buffer || d.buffer_size == 0) return false;
    if (s_text_input_count >= UIL_MAX_TEXT_INPUTS) return false;

    uint64_t key = widget_key(d.id ? d.id : "input", d.index);
    WidgetState* st = widget_state(key);

    bool hovered = d.id ? uil_element_hovered(d.id, d.index) : false;
    size_t len = strlen(d.buffer);

    // Clicking inside focuses; clicking anywhere else blurs.
    if (s_input.mouse_clicked[0]) {
        if (hovered) {
            s_focus_key = key;
            if (st) st->caret = len;
        } else if (s_focus_key == key) {
            s_focus_key = 0;
        }
    }

    bool focused = (s_focus_key == key);
    bool changed = false;

    if (focused && !d.readonly && st) {
        if (st->caret > len) st->caret = len;

        // Text insertion.
        const char* typed = input_text();
        size_t tn = strlen(typed);
        if (tn > 0 && len + tn < d.buffer_size) {
            memmove(d.buffer + st->caret + tn, d.buffer + st->caret, len - st->caret + 1);
            memcpy(d.buffer + st->caret, typed, tn);
            st->caret += tn;
            len += tn;
            changed = true;
        }

        // Backspace deletes a whole UTF-8 sequence, not one byte.
        if (input_key_pressed(INPUT_KEY_BACKSPACE) && st->caret > 0) {
            size_t prev = utf8_prev_boundary(d.buffer, st->caret);
            memmove(d.buffer + prev, d.buffer + st->caret, len - st->caret + 1);
            st->caret = prev;
            changed = true;
        }
        if (input_key_pressed(INPUT_KEY_DELETE) && st->caret < len) {
            size_t next = utf8_next_boundary(d.buffer, len, st->caret);
            memmove(d.buffer + st->caret, d.buffer + next, len - next + 1);
            changed = true;
        }

        if (input_key_pressed(INPUT_KEY_LEFT) && st->caret > 0) {
            st->caret = utf8_prev_boundary(d.buffer, st->caret);
        }
        if (input_key_pressed(INPUT_KEY_RIGHT) && st->caret < len) {
            st->caret = utf8_next_boundary(d.buffer, len, st->caret);
        }
        if (input_key_pressed(INPUT_KEY_HOME)) st->caret = 0;
        if (input_key_pressed(INPUT_KEY_END))  st->caret = strlen(d.buffer);
        if (input_key_pressed(INPUT_KEY_ESCAPE) || input_key_pressed(INPUT_KEY_ENTER)) {
            s_focus_key = 0;
        }
    }

    TextInputBlock* block = &s_text_inputs[s_text_input_count++];
    block->desc = d;
    block->key = key;
    block->focused = focused;

    Font* font = font_for(d.font);
    float h = d.h.kind ? 0.0f : 0.0f;
    (void)h;

    UiBoxDesc box = {
        .id = d.id, .index = d.index,
        .w = d.w.kind ? d.w : UI_GROW,
        .h = d.h.kind ? d.h : UI_PX(font_get_line_height(font) + 10.0f),
        .pad_x = d.pad_x > 0.0f ? d.pad_x : 6.0f,
        .align_y = UI_CENTER,
        .bg = d.bg,
        .radius = d.radius,
        .border = d.border > 0.0f ? d.border : 1.0f,
        .border_color = focused && d.border_focus_color.a > 0.0f ? d.border_focus_color
                                                                : d.border_color,
        .clip_x = true,
    };

    uil__open(0, box);
    uil_custom(UI_GROW, UI_GROW, draw_text_input, block);
    uil__close();

    Rect solved = d.id ? uil_element_rect(d.id, d.index) : rect_make(0, 0, 0, 0);
    if (solved.w > 0.0f) update_input_scroll(block, solved.w);

    return changed;
}

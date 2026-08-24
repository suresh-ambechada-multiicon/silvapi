#define CLAY_IMPLEMENTATION
#include "clay.h"
#include "clay_renderer.h"
#include "ui.h"
#include "icon.h"
#include <stdio.h>
#include <stdlib.h>

static void* s_clay_memory = NULL;
static size_t s_clay_memory_size = 0;

static void clay_error_handler(Clay_ErrorData errorText) {
    fprintf(stderr, "[Clay Error] %.*s\n", (int)errorText.errorText.length, errorText.errorText.chars);
}

static Clay_Dimensions clay_measure_text(Clay_StringSlice text, Clay_TextElementConfig *config, void *userData) {
    (void)userData;
    Font* font = g_ui.font_ui;
    if (config && config->fontId == 1) {
        font = g_ui.font_mono;
    } else if (config && config->fontId == 2) {
        font = g_ui.font_bold;
    }

    float w = 0.0f;
    float h = 16.0f;

    if (font && text.chars && text.length > 0) {
        w = font_measure_text(font, text.chars, text.length);
        h = font_get_height(font);
    }

    if (config && config->lineHeight > 0) {
        h = (float)config->lineHeight;
    }

    return (Clay_Dimensions){ .width = w, .height = h };
}

void clay_renderer_init(int window_w, int window_h) {
    s_clay_memory_size = Clay_MinMemorySize();
    s_clay_memory = malloc(s_clay_memory_size);
    if (!s_clay_memory) {
        fprintf(stderr, "Failed to allocate memory arena for Clay layout engine!\n");
        return;
    }

    Clay_Arena arena = Clay_CreateArenaWithCapacityAndMemory(s_clay_memory_size, s_clay_memory);
    Clay_Initialize(arena, (Clay_Dimensions){ (float)window_w, (float)window_h }, (Clay_ErrorHandler){ .errorHandlerFunction = clay_error_handler });
    Clay_SetMeasureTextFunction(clay_measure_text, NULL);
}

void clay_renderer_shutdown(void) {
    if (s_clay_memory) {
        free(s_clay_memory);
        s_clay_memory = NULL;
        s_clay_memory_size = 0;
    }
}

void clay_renderer_begin_frame(int window_w, int window_h, float dt) {
    Clay_SetLayoutDimensions((Clay_Dimensions){ (float)window_w, (float)window_h });
    Clay_SetPointerState((Clay_Vector2){ g_ui.mouse_x, g_ui.mouse_y }, g_ui.mouse_down[0]);
    Clay_UpdateScrollContainers(true, (Clay_Vector2){ g_ui.scroll_x, g_ui.scroll_y }, dt);
    Clay_BeginLayout();
}

void clay_renderer_end_and_render(float dt) {
    Clay_RenderCommandArray commands = Clay_EndLayout(dt);

    for (int32_t i = 0; i < commands.length; i++) {
        Clay_RenderCommand* cmd = Clay_RenderCommandArray_Get(&commands, i);
        if (!cmd) continue;

        switch (cmd->commandType) {
            case CLAY_RENDER_COMMAND_TYPE_RECTANGLE: {
                Clay_RectangleRenderData* data = &cmd->renderData.rectangle;
                Color col = color_from_clay(data->backgroundColor);
                float radius = data->cornerRadius.topLeft;
                Rect r = rect_make(cmd->boundingBox.x, cmd->boundingBox.y, cmd->boundingBox.width, cmd->boundingBox.height);
                if (radius > 0.0f) {
                    r_draw_rect_rounded(r, radius, col);
                } else {
                    r_draw_rect(r, col);
                }
                break;
            }

            case CLAY_RENDER_COMMAND_TYPE_BORDER: {
                Clay_BorderRenderData* data = &cmd->renderData.border;
                Color col = color_from_clay(data->color);
                float radius = data->cornerRadius.topLeft;
                float bdr_w = (float)data->width.top;
                Rect r = rect_make(cmd->boundingBox.x, cmd->boundingBox.y, cmd->boundingBox.width, cmd->boundingBox.height);
                r_draw_rect_rounded_bordered(r, radius, bdr_w > 0.0f ? bdr_w : 1.0f, col, (Color){0, 0, 0, 0});
                break;
            }

            case CLAY_RENDER_COMMAND_TYPE_TEXT: {
                Clay_TextRenderData* data = &cmd->renderData.text;
                Font* font = g_ui.font_ui;
                if (data->fontId == 1) font = g_ui.font_mono;
                else if (data->fontId == 2) font = g_ui.font_bold;

                Color col = color_from_clay(data->textColor);
                font_draw_text(font, data->stringContents.chars, data->stringContents.length, cmd->boundingBox.x, cmd->boundingBox.y, col);
                break;
            }

            case CLAY_RENDER_COMMAND_TYPE_IMAGE: {
                Clay_ImageRenderData* data = &cmd->renderData.image;
                if (data->imageData) {
                    IconType icon = *(IconType*)data->imageData;
                    Color col = color_from_clay(data->backgroundColor);
                    if (col.a <= 0.0f) col = (Color){1.0f, 1.0f, 1.0f, 1.0f};
                    Rect r = rect_make(cmd->boundingBox.x, cmd->boundingBox.y, cmd->boundingBox.width, cmd->boundingBox.height);
                    r_draw_icon(icon, r, col);
                }
                break;
            }

            case CLAY_RENDER_COMMAND_TYPE_SCISSOR_START: {
                r_push_clip(rect_make(cmd->boundingBox.x, cmd->boundingBox.y, cmd->boundingBox.width, cmd->boundingBox.height));
                break;
            }

            case CLAY_RENDER_COMMAND_TYPE_SCISSOR_END: {
                r_pop_clip();
                break;
            }

            default:
                break;
        }
    }
}

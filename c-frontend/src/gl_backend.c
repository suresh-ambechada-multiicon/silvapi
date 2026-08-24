#include "gl_backend.h"
#include <GL/gl.h>
#include <GL/glext.h>
#include "GLFW/glfw3.h"

// Define function pointers for modern GL
typedef GLuint (*PFNGLCREATESHADERPROC)(GLenum type);
typedef void (*PFNGLSHADERSOURCEPROC)(GLuint shader, GLsizei count, const GLchar *const*string, const GLint *length);
typedef void (*PFNGLCOMPILESHADERPROC)(GLuint shader);
typedef void (*PFNGLGETSHADERIVPROC)(GLuint shader, GLenum pname, GLint *params);
typedef void (*PFNGLGETSHADERINFOLOGPROC)(GLuint shader, GLsizei bufSize, GLsizei *length, GLchar *infoLog);
typedef GLuint (*PFNGLCREATEPROGRAMPROC)(void);
typedef void (*PFNGLATTACHSHADERPROC)(GLuint program, GLuint shader);
typedef void (*PFNGLLINKPROGRAMPROC)(GLuint program);
typedef void (*PFNGLGETPROGRAMIVPROC)(GLuint program, GLenum pname, GLint *params);
typedef void (*PFNGLGETPROGRAMINFOLOGPROC)(GLuint program, GLsizei bufSize, GLsizei *length, GLchar *infoLog);
typedef void (*PFNGLUSEPROGRAMPROC)(GLuint program);
typedef void (*PFNGLDELETESHADERPROC)(GLuint shader);
typedef void (*PFNGLDELETEPROGRAMPROC)(GLuint program);
typedef GLint (*PFNGLGETUNIFORMLOCATIONPROC)(GLuint program, const GLchar *name);
typedef void (*PFNGLUNIFORMMATRIX4FVPROC)(GLint location, GLsizei count, GLboolean transpose, const GLfloat *value);
typedef void (*PFNGLUNIFORM1IPROC)(GLint location, GLint v0);
typedef void (*PFNGLUNIFORM4FPROC)(GLint location, GLfloat v0, GLfloat v1, GLfloat v2, GLfloat v3);
typedef void (*PFNGLGENVERTEXARRAYSPROC)(GLsizei n, GLuint *arrays);
typedef void (*PFNGLBINDVERTEXARRAYPROC)(GLuint array);
typedef void (*PFNGLDELETEVERTEXARRAYSPROC)(GLsizei n, const GLuint *arrays);
typedef void (*PFNGLGENBUFFERSPROC)(GLsizei n, GLuint *buffers);
typedef void (*PFNGLBINDBUFFERPROC)(GLenum target, GLuint buffer);
typedef void (*PFNGLBUFFERDATAPROC)(GLenum target, GLsizeiptr size, const void *data, GLenum usage);
typedef void (*PFNGLBUFFERSUBDATAPROC)(GLenum target, GLintptr offset, GLsizeiptr size, const void *data);
typedef void (*PFNGLDELETEBUFFERSPROC)(GLsizei n, const GLuint *buffers);
typedef void (*PFNGLENABLEVERTEXATTRIBARRAYPROC)(GLuint index);
typedef void (*PFNGLVERTEXATTRIBPOINTERPROC)(GLuint index, GLint size, GLenum type, GLboolean normalized, GLsizei stride, const void *pointer);
typedef void (*PFNGLACTIVETEXTUREPROC)(GLenum texture);

static PFNGLCREATESHADERPROC glCreateShader_ptr = NULL;
static PFNGLSHADERSOURCEPROC glShaderSource_ptr = NULL;
static PFNGLCOMPILESHADERPROC glCompileShader_ptr = NULL;
static PFNGLGETSHADERIVPROC glGetShaderiv_ptr = NULL;
static PFNGLGETSHADERINFOLOGPROC glGetShaderInfoLog_ptr = NULL;
static PFNGLCREATEPROGRAMPROC glCreateProgram_ptr = NULL;
static PFNGLATTACHSHADERPROC glAttachShader_ptr = NULL;
static PFNGLLINKPROGRAMPROC glLinkProgram_ptr = NULL;
static PFNGLGETPROGRAMIVPROC glGetProgramiv_ptr = NULL;
static PFNGLGETPROGRAMINFOLOGPROC glGetProgramInfoLog_ptr = NULL;
static PFNGLUSEPROGRAMPROC glUseProgram_ptr = NULL;
static PFNGLDELETESHADERPROC glDeleteShader_ptr = NULL;
static PFNGLDELETEPROGRAMPROC glDeleteProgram_ptr = NULL;
static PFNGLGETUNIFORMLOCATIONPROC glGetUniformLocation_ptr = NULL;
static PFNGLUNIFORMMATRIX4FVPROC glUniformMatrix4fv_ptr = NULL;
static PFNGLUNIFORM1IPROC glUniform1i_ptr = NULL;
static PFNGLGENVERTEXARRAYSPROC glGenVertexArrays_ptr = NULL;
static PFNGLBINDVERTEXARRAYPROC glBindVertexArray_ptr = NULL;
static PFNGLDELETEVERTEXARRAYSPROC glDeleteVertexArrays_ptr = NULL;
static PFNGLGENBUFFERSPROC glGenBuffers_ptr = NULL;
static PFNGLBINDBUFFERPROC glBindBuffer_ptr = NULL;
static PFNGLBUFFERDATAPROC glBufferData_ptr = NULL;
static PFNGLBUFFERSUBDATAPROC glBufferSubData_ptr = NULL;
static PFNGLDELETEBUFFERSPROC glDeleteBuffers_ptr = NULL;
static PFNGLENABLEVERTEXATTRIBARRAYPROC glEnableVertexAttribArray_ptr = NULL;
static PFNGLVERTEXATTRIBPOINTERPROC glVertexAttribPointer_ptr = NULL;
static PFNGLACTIVETEXTUREPROC glActiveTexture_ptr = NULL;

#define MAX_BATCH_VERTICES 65536

typedef struct {
    float x, y;
    float u, v;
    float r, g, b, a;
    float corner_radius;
    float rect_w, rect_h;
    float local_x, local_y;
    float mode; // 0 = flat rect/rounded, 1 = textured quad
} Vertex2D;

static Vertex2D s_vertices[MAX_BATCH_VERTICES];
static size_t s_vertex_count = 0;
static GLuint s_vao = 0;
static GLuint s_vbo = 0;
static GLuint s_shader_program = 0;
static GLint s_u_projection = -1;
static GLint s_u_texture = -1;
static GLuint s_white_texture = 0;
static GLuint s_current_texture = 0;

static int s_window_w = 1280;
static int s_window_h = 720;

#define MAX_CLIP_STACK 64
static Rect s_clip_stack[MAX_CLIP_STACK];
static int s_clip_count = 0;

static const char* s_vs_source =
"#version 130\n"
"in vec2 a_pos;\n"
"in vec2 a_uv;\n"
"in vec4 a_color;\n"
"in float a_radius;\n"
"in vec2 a_rect_size;\n"
"in vec2 a_local_pos;\n"
"in float a_mode;\n"
"uniform mat4 u_projection;\n"
"out vec2 v_uv;\n"
"out vec4 v_color;\n"
"out float v_radius;\n"
"out vec2 v_rect_size;\n"
"out vec2 v_local_pos;\n"
"out float v_mode;\n"
"void main() {\n"
"    v_uv = a_uv;\n"
"    v_color = a_color;\n"
"    v_radius = a_radius;\n"
"    v_rect_size = a_rect_size;\n"
"    v_local_pos = a_local_pos;\n"
"    v_mode = a_mode;\n"
"    gl_Position = u_projection * vec4(a_pos, 0.0, 1.0);\n"
"}\n";

static const char* s_fs_source =
"#version 130\n"
"in vec2 v_uv;\n"
"in vec4 v_color;\n"
"in float v_radius;\n"
"in vec2 v_rect_size;\n"
"in vec2 v_local_pos;\n"
"in float v_mode;\n"
"uniform sampler2D u_texture;\n"
"out vec4 frag_color;\n"
"float rounded_box_sdf(vec2 center_pos, vec2 half_size, float radius) {\n"
"    vec2 d = abs(center_pos) - half_size + vec2(radius);\n"
"    return min(max(d.x, d.y), 0.0) + length(max(d, 0.0)) - radius;\n"
"}\n"
"void main() {\n"
"    if (v_mode > 0.5) {\n"
"        vec4 tex_color = texture2D(u_texture, v_uv);\n"
"        frag_color = v_color * tex_color;\n"
"    } else {\n"
"        if (v_radius > 0.0) {\n"
"            vec2 half_size = v_rect_size * 0.5;\n"
"            vec2 center_pos = v_local_pos - half_size;\n"
"            float dist = rounded_box_sdf(center_pos, half_size, v_radius);\n"
"            float alpha = clamp(0.5 - dist, 0.0, 1.0);\n"
"            frag_color = vec4(v_color.rgb, v_color.a * alpha);\n"
"        } else {\n"
"            frag_color = v_color;\n"
"        }\n"
"    }\n"
"    if (frag_color.a <= 0.001) discard;\n"
"}\n";

static GLuint compile_shader(GLenum type, const char* source) {
    GLuint shader = glCreateShader_ptr(type);
    glShaderSource_ptr(shader, 1, &source, NULL);
    glCompileShader_ptr(shader);
    GLint status;
    glGetShaderiv_ptr(shader, GL_COMPILE_STATUS, &status);
    if (!status) {
        char log[1024];
        glGetShaderInfoLog_ptr(shader, sizeof(log), NULL, log);
        fprintf(stderr, "Shader compile error: %s\n", log);
        glDeleteShader_ptr(shader);
        return 0;
    }
    return shader;
}

static void flush_batch(void) {
    if (s_vertex_count == 0) return;

    glUseProgram_ptr(s_shader_program);
    glBindVertexArray_ptr(s_vao);
    glBindBuffer_ptr(GL_ARRAY_BUFFER, s_vbo);
    glBufferSubData_ptr(GL_ARRAY_BUFFER, 0, sizeof(Vertex2D) * s_vertex_count, s_vertices);

    glActiveTexture_ptr(GL_TEXTURE0);
    glBindTexture(GL_TEXTURE_2D, s_current_texture ? s_current_texture : s_white_texture);
    glUniform1i_ptr(s_u_texture, 0);

    glDrawArrays(GL_TRIANGLES, 0, (GLsizei)s_vertex_count);

    s_vertex_count = 0;
}

static void set_current_texture(GLuint tex) {
    if (s_current_texture != tex) {
        flush_batch();
        s_current_texture = tex;
    }
}

static void update_clip_scissor(void) {
    flush_batch();
    if (s_clip_count == 0) {
        glDisable(GL_SCISSOR_TEST);
    } else {
        glEnable(GL_SCISSOR_TEST);
        Rect r = s_clip_stack[s_clip_count - 1];
        // Convert top-left coordinates to bottom-left for OpenGL
        int sx = (int)r.x;
        int sy = (int)((float)s_window_h - (r.y + r.h));
        int sw = (int)r.w;
        int sh = (int)r.h;
        if (sw < 0) sw = 0;
        if (sh < 0) sh = 0;
        glScissor(sx, sy, sw, sh);
    }
}

void r_push_clip(Rect rect) {
    if (s_clip_count < MAX_CLIP_STACK) {
        if (s_clip_count > 0) {
            rect = rect_intersect(s_clip_stack[s_clip_count - 1], rect);
        }
        s_clip_stack[s_clip_count++] = rect;
        update_clip_scissor();
    }
}

void r_pop_clip(void) {
    if (s_clip_count > 0) {
        s_clip_count--;
        update_clip_scissor();
    }
}

Rect r_get_clip(void) {
    if (s_clip_count > 0) {
        return s_clip_stack[s_clip_count - 1];
    }
    return rect_make(0, 0, (float)s_window_w, (float)s_window_h);
}

bool gl_backend_init(int window_width, int window_height) {
    s_window_w = window_width;
    s_window_h = window_height;

    // Load function pointers via GLFW
    glCreateShader_ptr = (PFNGLCREATESHADERPROC)glfwGetProcAddress("glCreateShader");
    glShaderSource_ptr = (PFNGLSHADERSOURCEPROC)glfwGetProcAddress("glShaderSource");
    glCompileShader_ptr = (PFNGLCOMPILESHADERPROC)glfwGetProcAddress("glCompileShader");
    glGetShaderiv_ptr = (PFNGLGETSHADERIVPROC)glfwGetProcAddress("glGetShaderiv");
    glGetShaderInfoLog_ptr = (PFNGLGETSHADERINFOLOGPROC)glfwGetProcAddress("glGetShaderInfoLog");
    glCreateProgram_ptr = (PFNGLCREATEPROGRAMPROC)glfwGetProcAddress("glCreateProgram");
    glAttachShader_ptr = (PFNGLATTACHSHADERPROC)glfwGetProcAddress("glAttachShader");
    glLinkProgram_ptr = (PFNGLLINKPROGRAMPROC)glfwGetProcAddress("glLinkProgram");
    glGetProgramiv_ptr = (PFNGLGETPROGRAMIVPROC)glfwGetProcAddress("glGetProgramiv");
    glGetProgramInfoLog_ptr = (PFNGLGETPROGRAMINFOLOGPROC)glfwGetProcAddress("glGetProgramInfoLog");
    glUseProgram_ptr = (PFNGLUSEPROGRAMPROC)glfwGetProcAddress("glUseProgram");
    glDeleteShader_ptr = (PFNGLDELETESHADERPROC)glfwGetProcAddress("glDeleteShader");
    glDeleteProgram_ptr = (PFNGLDELETEPROGRAMPROC)glfwGetProcAddress("glDeleteProgram");
    glGetUniformLocation_ptr = (PFNGLGETUNIFORMLOCATIONPROC)glfwGetProcAddress("glGetUniformLocation");
    glUniformMatrix4fv_ptr = (PFNGLUNIFORMMATRIX4FVPROC)glfwGetProcAddress("glUniformMatrix4fv");
    glUniform1i_ptr = (PFNGLUNIFORM1IPROC)glfwGetProcAddress("glUniform1i");
    glGenVertexArrays_ptr = (PFNGLGENVERTEXARRAYSPROC)glfwGetProcAddress("glGenVertexArrays");
    glBindVertexArray_ptr = (PFNGLBINDVERTEXARRAYPROC)glfwGetProcAddress("glBindVertexArray");
    glDeleteVertexArrays_ptr = (PFNGLDELETEVERTEXARRAYSPROC)glfwGetProcAddress("glDeleteVertexArrays");
    glGenBuffers_ptr = (PFNGLGENBUFFERSPROC)glfwGetProcAddress("glGenBuffers");
    glBindBuffer_ptr = (PFNGLBINDBUFFERPROC)glfwGetProcAddress("glBindBuffer");
    glBufferData_ptr = (PFNGLBUFFERDATAPROC)glfwGetProcAddress("glBufferData");
    glBufferSubData_ptr = (PFNGLBUFFERSUBDATAPROC)glfwGetProcAddress("glBufferSubData");
    glDeleteBuffers_ptr = (PFNGLDELETEBUFFERSPROC)glfwGetProcAddress("glDeleteBuffers");
    glEnableVertexAttribArray_ptr = (PFNGLENABLEVERTEXATTRIBARRAYPROC)glfwGetProcAddress("glEnableVertexAttribArray");
    glVertexAttribPointer_ptr = (PFNGLVERTEXATTRIBPOINTERPROC)glfwGetProcAddress("glVertexAttribPointer");
    glActiveTexture_ptr = (PFNGLACTIVETEXTUREPROC)glfwGetProcAddress("glActiveTexture");

    if (!glCreateShader_ptr || !glGenBuffers_ptr) {
        fprintf(stderr, "Failed to load OpenGL shader/buffer functions!\n");
        return false;
    }

    // Compile Shader
    GLuint vs = compile_shader(GL_VERTEX_SHADER, s_vs_source);
    GLuint fs = compile_shader(GL_FRAGMENT_SHADER, s_fs_source);
    if (!vs || !fs) return false;

    s_shader_program = glCreateProgram_ptr();
    glAttachShader_ptr(s_shader_program, vs);
    glAttachShader_ptr(s_shader_program, fs);
    glLinkProgram_ptr(s_shader_program);

    GLint status;
    glGetProgramiv_ptr(s_shader_program, GL_LINK_STATUS, &status);
    if (!status) {
        char log[1024];
        glGetProgramInfoLog_ptr(s_shader_program, sizeof(log), NULL, log);
        fprintf(stderr, "Program link error: %s\n", log);
        return false;
    }

    glDeleteShader_ptr(vs);
    glDeleteShader_ptr(fs);

    s_u_projection = glGetUniformLocation_ptr(s_shader_program, "u_projection");
    s_u_texture = glGetUniformLocation_ptr(s_shader_program, "u_texture");

    // VAO / VBO
    glGenVertexArrays_ptr(1, &s_vao);
    glBindVertexArray_ptr(s_vao);

    glGenBuffers_ptr(1, &s_vbo);
    glBindBuffer_ptr(GL_ARRAY_BUFFER, s_vbo);
    glBufferData_ptr(GL_ARRAY_BUFFER, sizeof(Vertex2D) * MAX_BATCH_VERTICES, NULL, GL_DYNAMIC_DRAW);

    // Layout
    size_t stride = sizeof(Vertex2D);
    glEnableVertexAttribArray_ptr(0); // a_pos
    glVertexAttribPointer_ptr(0, 2, GL_FLOAT, GL_FALSE, (GLsizei)stride, (void*)0);
    glEnableVertexAttribArray_ptr(1); // a_uv
    glVertexAttribPointer_ptr(1, 2, GL_FLOAT, GL_FALSE, (GLsizei)stride, (void*)(2 * sizeof(float)));
    glEnableVertexAttribArray_ptr(2); // a_color
    glVertexAttribPointer_ptr(2, 4, GL_FLOAT, GL_FALSE, (GLsizei)stride, (void*)(4 * sizeof(float)));
    glEnableVertexAttribArray_ptr(3); // a_radius
    glVertexAttribPointer_ptr(3, 1, GL_FLOAT, GL_FALSE, (GLsizei)stride, (void*)(8 * sizeof(float)));
    glEnableVertexAttribArray_ptr(4); // a_rect_size
    glVertexAttribPointer_ptr(4, 2, GL_FLOAT, GL_FALSE, (GLsizei)stride, (void*)(9 * sizeof(float)));
    glEnableVertexAttribArray_ptr(5); // a_local_pos
    glVertexAttribPointer_ptr(5, 2, GL_FLOAT, GL_FALSE, (GLsizei)stride, (void*)(11 * sizeof(float)));
    glEnableVertexAttribArray_ptr(6); // a_mode
    glVertexAttribPointer_ptr(6, 1, GL_FLOAT, GL_FALSE, (GLsizei)stride, (void*)(13 * sizeof(float)));

    glBindVertexArray_ptr(0);

    // 1x1 White Texture
    uint32_t white_pixel = 0xFFFFFFFF;
    glGenTextures(1, &s_white_texture);
    glBindTexture(GL_TEXTURE_2D, s_white_texture);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, 1, 1, 0, GL_RGBA, GL_UNSIGNED_BYTE, &white_pixel);

    return true;
}

void gl_backend_shutdown(void) {
    if (s_white_texture) glDeleteTextures(1, &s_white_texture);
    if (s_vbo) glDeleteBuffers_ptr(1, &s_vbo);
    if (s_vao) glDeleteVertexArrays_ptr(1, &s_vao);
    if (s_shader_program) glDeleteProgram_ptr(s_shader_program);
}

void gl_backend_resize(int width, int height) {
    s_window_w = width;
    s_window_h = height;
    glViewport(0, 0, width, height);
}

void gl_backend_begin_frame(void) {
    s_vertex_count = 0;
    s_current_texture = 0;
    s_clip_count = 0;

    glDisable(GL_SCISSOR_TEST);
    glEnable(GL_BLEND);
    glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);

    // Set ortho projection matrix: (0, 0) top-left to (w, h) bottom-right
    float L = 0.0f;
    float R = (float)s_window_w;
    float T = 0.0f;
    float B = (float)s_window_h;
    float N = -1.0f;
    float F = 1.0f;

    float ortho[16] = {
        2.0f / (R - L), 0.0f, 0.0f, 0.0f,
        0.0f, 2.0f / (T - B), 0.0f, 0.0f,
        0.0f, 0.0f, -2.0f / (F - N), 0.0f,
        -(R + L) / (R - L), -(T + B) / (T - B), -(F + N) / (F - N), 1.0f
    };

    glUseProgram_ptr(s_shader_program);
    glUniformMatrix4fv_ptr(s_u_projection, 1, GL_FALSE, ortho);
}

void gl_backend_end_frame(void) {
    flush_batch();
    glDisable(GL_SCISSOR_TEST);
}

static void push_quad(float x0, float y0, float x1, float y1,
                      float u0, float v0, float u1, float v1,
                      Color c, float radius, float rw, float rh, float mode) {
    if (s_vertex_count + 6 >= MAX_BATCH_VERTICES) {
        flush_batch();
    }

    Vertex2D vert0 = {x0, y0, u0, v0, c.r, c.g, c.b, c.a, radius, rw, rh, 0.0f, 0.0f, mode};
    Vertex2D vert1 = {x1, y0, u1, v0, c.r, c.g, c.b, c.a, radius, rw, rh, rw, 0.0f, mode};
    Vertex2D vert2 = {x1, y1, u1, v1, c.r, c.g, c.b, c.a, radius, rw, rh, rw, rh, mode};
    Vertex2D vert3 = {x0, y1, u0, v1, c.r, c.g, c.b, c.a, radius, rw, rh, 0.0f, rh, mode};

    s_vertices[s_vertex_count++] = vert0;
    s_vertices[s_vertex_count++] = vert1;
    s_vertices[s_vertex_count++] = vert2;
    s_vertices[s_vertex_count++] = vert0;
    s_vertices[s_vertex_count++] = vert2;
    s_vertices[s_vertex_count++] = vert3;
}

void r_draw_rect(Rect rect, Color color) {
    set_current_texture(0);
    push_quad(rect.x, rect.y, rect.x + rect.w, rect.y + rect.h,
              0.0f, 0.0f, 1.0f, 1.0f,
              color, 0.0f, rect.w, rect.h, 0.0f);
}

void r_draw_rect_rounded(Rect rect, float radius, Color color) {
    set_current_texture(0);
    push_quad(rect.x, rect.y, rect.x + rect.w, rect.y + rect.h,
              0.0f, 0.0f, 1.0f, 1.0f,
              color, radius, rect.w, rect.h, 0.0f);
}

void r_draw_rect_bordered(Rect rect, float border_width, Color border_color, Color fill_color) {
    if (fill_color.a > 0.001f) {
        r_draw_rect(rect, fill_color);
    }
    if (border_width > 0.0f && border_color.a > 0.001f) {
        // Top, bottom, left, right borders
        r_draw_rect(rect_make(rect.x, rect.y, rect.w, border_width), border_color);
        r_draw_rect(rect_make(rect.x, rect.y + rect.h - border_width, rect.w, border_width), border_color);
        r_draw_rect(rect_make(rect.x, rect.y + border_width, border_width, rect.h - 2.0f * border_width), border_color);
        r_draw_rect(rect_make(rect.x + rect.w - border_width, rect.y + border_width, border_width, rect.h - 2.0f * border_width), border_color);
    }
}

void r_draw_rect_rounded_bordered(Rect rect, float radius, float border_width, Color border_color, Color fill_color) {
    if (border_width <= 0.0f || border_color.a < 0.001f) {
        r_draw_rect_rounded(rect, radius, fill_color);
        return;
    }
    // Draw outer rounded border
    r_draw_rect_rounded(rect, radius, border_color);
    // Draw inner fill
    float inner_r = radius > border_width ? radius - border_width : 0.0f;
    r_draw_rect_rounded(rect_make(rect.x + border_width, rect.y + border_width,
                                  rect.w - 2.0f * border_width, rect.h - 2.0f * border_width),
                        inner_r, fill_color);
}

void r_draw_line(Vec2 p1, Vec2 p2, float thickness, Color color) {
    set_current_texture(0);
    float dx = p2.x - p1.x;
    float dy = p2.y - p1.y;
    float len = sqrtf(dx * dx + dy * dy);
    if (len < 0.0001f) return;
    
    float nx = -dy / len * (thickness * 0.5f);
    float ny = dx / len * (thickness * 0.5f);

    if (s_vertex_count + 6 >= MAX_BATCH_VERTICES) {
        flush_batch();
    }

    Vertex2D v0 = {p1.x + nx, p1.y + ny, 0, 0, color.r, color.g, color.b, color.a, 0, 0, 0, 0, 0, 0};
    Vertex2D v1 = {p2.x + nx, p2.y + ny, 0, 0, color.r, color.g, color.b, color.a, 0, 0, 0, 0, 0, 0};
    Vertex2D v2 = {p2.x - nx, p2.y - ny, 0, 0, color.r, color.g, color.b, color.a, 0, 0, 0, 0, 0, 0};
    Vertex2D v3 = {p1.x - nx, p1.y - ny, 0, 0, color.r, color.g, color.b, color.a, 0, 0, 0, 0, 0, 0};

    s_vertices[s_vertex_count++] = v0;
    s_vertices[s_vertex_count++] = v1;
    s_vertices[s_vertex_count++] = v2;
    s_vertices[s_vertex_count++] = v0;
    s_vertices[s_vertex_count++] = v2;
    s_vertices[s_vertex_count++] = v3;
}

void r_draw_textured_quad(Rect dst, Rect src_uv, uint32_t texture_id, Color tint) {
    set_current_texture(texture_id);
    push_quad(dst.x, dst.y, dst.x + dst.w, dst.y + dst.h,
              src_uv.x, src_uv.y, src_uv.x + src_uv.w, src_uv.y + src_uv.h,
              tint, 0.0f, dst.w, dst.h, 1.0f);
}

// Procedural vector icon drawing
void r_draw_icon(IconType icon, Rect rect, Color color) {
    float cx = rect.x + rect.w * 0.5f;
    float cy = rect.y + rect.h * 0.5f;
    float s = fminf(rect.w, rect.h) * 0.45f;

    switch (icon) {
        case ICON_SEARCH: {
            float r = s * 0.65f;
            Vec2 center = {cx - s * 0.2f, cy - s * 0.2f};
            // Circle ring approximation
            const int segs = 12;
            for (int i = 0; i < segs; i++) {
                float a1 = (float)i / segs * 6.283185f;
                float a2 = (float)(i + 1) / segs * 6.283185f;
                r_draw_line((Vec2){center.x + cosf(a1)*r, center.y + sinf(a1)*r},
                            (Vec2){center.x + cosf(a2)*r, center.y + sinf(a2)*r}, 1.5f, color);
            }
            r_draw_line((Vec2){center.x + r * 0.7f, center.y + r * 0.7f},
                        (Vec2){cx + s * 0.8f, cy + s * 0.8f}, 2.0f, color);
            break;
        }
        case ICON_PLUS: {
            r_draw_line((Vec2){cx - s, cy}, (Vec2){cx + s, cy}, 1.5f, color);
            r_draw_line((Vec2){cx, cy - s}, (Vec2){cx, cy + s}, 1.5f, color);
            break;
        }
        case ICON_CLOSE: {
            float d = s * 0.7f;
            r_draw_line((Vec2){cx - d, cy - d}, (Vec2){cx + d, cy + d}, 1.5f, color);
            r_draw_line((Vec2){cx + d, cy - d}, (Vec2){cx - d, cy + d}, 1.5f, color);
            break;
        }
        case ICON_PLAY: {
            float d = s * 0.8f;
            Vec2 p0 = {cx - d * 0.6f, cy - d};
            Vec2 p1 = {cx + d * 0.8f, cy};
            Vec2 p2 = {cx - d * 0.6f, cy + d};
            r_draw_line(p0, p1, 1.5f, color);
            r_draw_line(p1, p2, 1.5f, color);
            r_draw_line(p2, p0, 1.5f, color);
            // Filled triangle
            set_current_texture(0);
            if (s_vertex_count + 3 < MAX_BATCH_VERTICES) {
                Vertex2D v0 = {p0.x, p0.y, 0,0, color.r, color.g, color.b, color.a, 0,0,0,0,0,0};
                Vertex2D v1 = {p1.x, p1.y, 0,0, color.r, color.g, color.b, color.a, 0,0,0,0,0,0};
                Vertex2D v2 = {p2.x, p2.y, 0,0, color.r, color.g, color.b, color.a, 0,0,0,0,0,0};
                s_vertices[s_vertex_count++] = v0;
                s_vertices[s_vertex_count++] = v1;
                s_vertices[s_vertex_count++] = v2;
            }
            break;
        }
        case ICON_STOP: {
            float d = s * 0.7f;
            r_draw_rect(rect_make(cx - d, cy - d, d * 2.0f, d * 2.0f), color);
            break;
        }
        case ICON_CHEVRON_RIGHT: {
            float d = s * 0.5f;
            r_draw_line((Vec2){cx - d * 0.5f, cy - d}, (Vec2){cx + d * 0.5f, cy}, 1.5f, color);
            r_draw_line((Vec2){cx + d * 0.5f, cy}, (Vec2){cx - d * 0.5f, cy + d}, 1.5f, color);
            break;
        }
        case ICON_CHEVRON_DOWN: {
            float d = s * 0.5f;
            r_draw_line((Vec2){cx - d, cy - d * 0.5f}, (Vec2){cx, cy + d * 0.5f}, 1.5f, color);
            r_draw_line((Vec2){cx, cy + d * 0.5f}, (Vec2){cx + d, cy - d * 0.5f}, 1.5f, color);
            break;
        }
        case ICON_FOLDER:
        case ICON_FOLDER_OPEN: {
            float fw = s * 1.8f;
            float fh = s * 1.3f;
            float fx = cx - fw * 0.5f;
            float fy = cy - fh * 0.5f;
            // Folder tab
            r_draw_rect(rect_make(fx, fy, fw * 0.45f, fh * 0.35f), color);
            // Folder body
            r_draw_rect_bordered(rect_make(fx, fy + fh * 0.25f, fw, fh * 0.75f), 1.0f, color, color_with_alpha(color, 0.15f));
            break;
        }
        case ICON_FILE: {
            float fw = s * 1.2f;
            float fh = s * 1.6f;
            float fx = cx - fw * 0.5f;
            float fy = cy - fh * 0.5f;
            r_draw_rect_bordered(rect_make(fx, fy, fw, fh), 1.0f, color, color_with_alpha(color, 0.1f));
            r_draw_line((Vec2){fx + fw * 0.25f, fy + fh * 0.35f}, (Vec2){fx + fw * 0.75f, fy + fh * 0.35f}, 1.0f, color);
            r_draw_line((Vec2){fx + fw * 0.25f, fy + fh * 0.6f}, (Vec2){fx + fw * 0.75f, fy + fh * 0.6f}, 1.0f, color);
            break;
        }
        case ICON_TRASH: {
            float w = s * 1.2f;
            float h = s * 1.4f;
            r_draw_line((Vec2){cx - w * 0.6f, cy - h * 0.5f}, (Vec2){cx + w * 0.6f, cy - h * 0.5f}, 1.5f, color);
            r_draw_rect_bordered(rect_make(cx - w * 0.4f, cy - h * 0.35f, w * 0.8f, h * 0.85f), 1.0f, color, color_with_alpha(color, 0.1f));
            break;
        }
        case ICON_COPY: {
            float d = s * 0.9f;
            r_draw_rect_bordered(rect_make(cx - d * 0.8f, cy - d * 0.8f, d * 1.1f, d * 1.1f), 1.0f, color, color_rgba(0,0,0,0));
            r_draw_rect_bordered(rect_make(cx - d * 0.3f, cy - d * 0.3f, d * 1.1f, d * 1.1f), 1.0f, color, color_with_alpha(color, 0.2f));
            break;
        }
        case ICON_CHECK: {
            Vec2 p0 = {cx - s * 0.7f, cy};
            Vec2 p1 = {cx - s * 0.1f, cy + s * 0.6f};
            Vec2 p2 = {cx + s * 0.8f, cy - s * 0.6f};
            r_draw_line(p0, p1, 2.0f, color);
            r_draw_line(p1, p2, 2.0f, color);
            break;
        }
        case ICON_SETTINGS: {
            float r = s * 0.7f;
            for (int i = 0; i < 6; i++) {
                float a = (float)i / 6.0f * 6.283185f;
                r_draw_line((Vec2){cx, cy}, (Vec2){cx + cosf(a)*r, cy + sinf(a)*r}, 2.0f, color);
            }
            r_draw_rect_rounded(rect_make(cx - r*0.4f, cy - r*0.4f, r*0.8f, r*0.8f), r*0.4f, color);
            break;
        }
        case ICON_VARIABLE: {
            // Draw {{ }} or {x} symbol
            float d = s * 0.8f;
            r_draw_line((Vec2){cx - d, cy - d}, (Vec2){cx - d * 0.5f, cy}, 1.5f, color);
            r_draw_line((Vec2){cx - d * 0.5f, cy}, (Vec2){cx - d, cy + d}, 1.5f, color);
            r_draw_line((Vec2){cx + d, cy - d}, (Vec2){cx + d * 0.5f, cy}, 1.5f, color);
            r_draw_line((Vec2){cx + d * 0.5f, cy}, (Vec2){cx + d, cy + d}, 1.5f, color);
            break;
        }
        case ICON_IMPORT: {
            float d = s * 0.8f;
            r_draw_line((Vec2){cx, cy - d}, (Vec2){cx, cy + d * 0.5f}, 1.5f, color);
            r_draw_line((Vec2){cx - d * 0.5f, cy}, (Vec2){cx, cy + d * 0.5f}, 1.5f, color);
            r_draw_line((Vec2){cx + d * 0.5f, cy}, (Vec2){cx, cy + d * 0.5f}, 1.5f, color);
            r_draw_line((Vec2){cx - d * 0.7f, cy + d}, (Vec2){cx + d * 0.7f, cy + d}, 1.5f, color);
            break;
        }
        case ICON_DOT: {
            float r = s * 0.4f;
            r_draw_rect_rounded(rect_make(cx - r, cy - r, r * 2.0f, r * 2.0f), r, color);
            break;
        }
        case ICON_REFRESH: {
            float r = s * 0.7f;
            const int segs = 10;
            for (int i = 0; i < segs; i++) {
                float a1 = (float)i / 12.0f * 6.283185f;
                float a2 = (float)(i + 1) / 12.0f * 6.283185f;
                r_draw_line((Vec2){cx + cosf(a1)*r, cy + sinf(a1)*r},
                            (Vec2){cx + cosf(a2)*r, cy + sinf(a2)*r}, 1.5f, color);
            }
            break;
        }
        default:
            break;
    }
}

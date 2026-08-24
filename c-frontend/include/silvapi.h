#ifndef SILVAPI_H
#define SILVAPI_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <stdint.h>
#include <math.h>
#include <time.h>
#include <pthread.h>

#define SILVAPI_VERSION "0.1.0"
#define SILVAPI_TITLE "Silvapi"

// Vector2 and Color definitions
typedef struct {
    float x, y;
} Vec2;

typedef struct {
    float x, y, w, h;
} Rect;

typedef struct {
    float r, g, b, a;
} Color;

static inline Color color_rgba(float r, float g, float b, float a) {
    Color c = {r, g, b, a};
    return c;
}

static inline Color color_rgb(float r, float g, float b) {
    Color c = {r, g, b, 1.0f};
    return c;
}

static inline Color color_hex(uint32_t hex) {
    Color c;
    if ((hex & 0xFF000000) != 0) {
        c.r = ((hex >> 24) & 0xFF) / 255.0f;
        c.g = ((hex >> 16) & 0xFF) / 255.0f;
        c.b = ((hex >> 8) & 0xFF) / 255.0f;
        c.a = (hex & 0xFF) / 255.0f;
    } else {
        c.r = ((hex >> 16) & 0xFF) / 255.0f;
        c.g = ((hex >> 8) & 0xFF) / 255.0f;
        c.b = (hex & 0xFF) / 255.0f;
        c.a = 1.0f;
    }
    return c;
}

static inline Color color_with_alpha(Color c, float a) {
    c.a = a;
    return c;
}

static inline Rect rect_make(float x, float y, float w, float h) {
    Rect r = {x, y, w, h};
    return r;
}

static inline bool rect_contains(Rect r, float x, float y) {
    return x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h;
}

static inline Rect rect_intersect(Rect a, Rect b) {
    float x1 = (a.x > b.x) ? a.x : b.x;
    float y1 = (a.y > b.y) ? a.y : b.y;
    float x2 = (a.x + a.w < b.x + b.w) ? (a.x + a.w) : (b.x + b.w);
    float y2 = (a.y + a.h < b.y + b.h) ? (a.y + a.h) : (b.y + b.h);
    if (x2 < x1 || y2 < y1) {
        Rect empty = {0, 0, 0, 0};
        return empty;
    }
    Rect res = {x1, y1, x2 - x1, y2 - y1};
    return res;
}

#endif // SILVAPI_H

#ifndef SPRING_H
#define SPRING_H

#include "base/base.h"
#include <stdbool.h>

// Second-Order Dynamic Spring Motion Engine (File Pilot fluid physics)
// Models natural harmonic oscillation, damping, and velocity tracking.

typedef struct {
    float f;      // Natural frequency (Hz / speed): higher = faster response (e.g. 10.0 - 24.0)
    float zeta;   // Damping ratio: 1.0 = critically damped, < 1.0 = bouncy, > 1.0 = overdamped
    float r;      // Initial acceleration response: 0 = smooth ramp, 1 = instant velocity reaction
} SpringConfig;

// Common Presets
#define SPRING_CONFIG_SNAPPY  (SpringConfig){.f = 18.0f, .zeta = 1.00f, .r = 0.0f}
#define SPRING_CONFIG_BOUNCY  (SpringConfig){.f = 14.0f, .zeta = 0.75f, .r = 0.0f}
#define SPRING_CONFIG_GLIDE   (SpringConfig){.f = 22.0f, .zeta = 0.90f, .r = 1.0f}
#define SPRING_CONFIG_SMOOTH  (SpringConfig){.f = 12.0f, .zeta = 1.00f, .r = 0.0f}
#define SPRING_CONFIG_GENTLE  (SpringConfig){.f = 8.0f,  .zeta = 1.00f, .r = 0.0f}

// 1-Dimensional Spring (scalars, scroll offsets, split ratios, opacities)
typedef struct {
    float pos;
    float vel;
    float target;
    SpringConfig config;
} Spring1D;

// 2-Dimensional Spring (coordinates, mouse followers, popovers)
typedef struct {
    Vec2 pos;
    Vec2 vel;
    Vec2 target;
    SpringConfig config;
} Spring2D;

// Rectangle Spring (docking preview boxes, selection bounds, tab indicators, modals)
typedef struct {
    Rect pos;
    Rect vel;
    Rect target;
    SpringConfig config;
} SpringRect;

// Color Spring (theme transitions, hover color blending)
typedef struct {
    Color pos;
    Color vel;
    Color target;
    SpringConfig config;
} SpringColor;

// File Pilot Gliding Caret Spring (dual leading/trailing springs with velocity stretch)
typedef struct {
    Spring1D lead_x;
    Spring1D trail_x;
    Spring1D pos_y;
    Spring1D height;
    float blink_time;
    bool is_typing;
} CaretSpring;

// -----------------------------------------------------------------------------
// Functions
// -----------------------------------------------------------------------------

// Spring 1D
Spring1D spring1d_make(float initial_val, SpringConfig cfg);
void     spring1d_set_target(Spring1D* s, float target);
void     spring1d_snap(Spring1D* s, float val);
void     spring1d_update(Spring1D* s, float dt);
bool     spring1d_is_settled(const Spring1D* s, float eps_pos, float eps_vel);

// Spring 2D
Spring2D spring2d_make(Vec2 initial_val, SpringConfig cfg);
void     spring2d_set_target(Spring2D* s, Vec2 target);
void     spring2d_snap(Spring2D* s, Vec2 val);
void     spring2d_update(Spring2D* s, float dt);
bool     spring2d_is_settled(const Spring2D* s, float eps_pos, float eps_vel);

// Spring Rect
SpringRect spring_rect_make(Rect initial_val, SpringConfig cfg);
void       spring_rect_set_target(SpringRect* s, Rect target);
void       spring_rect_snap(SpringRect* s, Rect val);
void       spring_rect_update(SpringRect* s, float dt);
bool       spring_rect_is_settled(const SpringRect* s, float eps_pos, float eps_vel);

// Spring Color
SpringColor spring_color_make(Color initial_val, SpringConfig cfg);
void        spring_color_set_target(SpringColor* s, Color target);
void        spring_color_snap(SpringColor* s, Color val);
void        spring_color_update(SpringColor* s, float dt);
bool        spring_color_is_settled(const SpringColor* s, float eps);

// Caret Spring (File Pilot Cursor)
CaretSpring caret_spring_make(float initial_x, float initial_y, float height);
void        caret_spring_set_target(CaretSpring* c, float target_x, float target_y, float height);
void        caret_spring_snap(CaretSpring* c, float target_x, float target_y, float height);
void        caret_spring_notify_typing(CaretSpring* c);
void        caret_spring_update(CaretSpring* c, float dt);
Rect        caret_spring_get_bounds(const CaretSpring* c, float min_caret_width);
float       caret_spring_get_alpha(const CaretSpring* c);

#endif // SPRING_H

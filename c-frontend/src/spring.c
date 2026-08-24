#include "spring.h"
#include <math.h>

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

#define MAX_SUBSTEP_DT 0.008333333f // Max 120Hz physics sub-step for numerical stability

static void spring1d_integrate(float* pos, float* vel, float target, SpringConfig cfg, float dt) {
    if (cfg.f <= 0.001f) {
        *pos = target;
        *vel = 0.0f;
        return;
    }

    float omega = 2.0f * (float)M_PI * cfg.f;
    float k1 = cfg.zeta / ((float)M_PI * cfg.f);
    float k2 = 1.0f / (omega * omega);
    float k3 = cfg.r * cfg.zeta / omega;

    // Sub-stepping for stability on large frame times
    while (dt > 0.0f) {
        float step_dt = (dt > MAX_SUBSTEP_DT) ? MAX_SUBSTEP_DT : dt;
        dt -= step_dt;

        float target_vel = 0.0f;
        float acc = (target + k3 * target_vel - *pos - k1 * *vel) / k2;

        *pos += step_dt * *vel;
        *vel += step_dt * acc;
    }
}

// -----------------------------------------------------------------------------
// Spring 1D
// -----------------------------------------------------------------------------

Spring1D spring1d_make(float initial_val, SpringConfig cfg) {
    Spring1D s;
    s.pos = initial_val;
    s.vel = 0.0f;
    s.target = initial_val;
    s.config = cfg;
    return s;
}

void spring1d_set_target(Spring1D* s, float target) {
    if (s) s->target = target;
}

void spring1d_snap(Spring1D* s, float val) {
    if (s) {
        s->pos = val;
        s->vel = 0.0f;
        s->target = val;
    }
}

void spring1d_update(Spring1D* s, float dt) {
    if (!s || dt <= 0.0f) return;
    spring1d_integrate(&s->pos, &s->vel, s->target, s->config, dt);
}

bool spring1d_is_settled(const Spring1D* s, float eps_pos, float eps_vel) {
    if (!s) return true;
    return (fabsf(s->pos - s->target) <= eps_pos) && (fabsf(s->vel) <= eps_vel);
}

// -----------------------------------------------------------------------------
// Spring 2D
// -----------------------------------------------------------------------------

Spring2D spring2d_make(Vec2 initial_val, SpringConfig cfg) {
    Spring2D s;
    s.pos = initial_val;
    s.vel = (Vec2){0.0f, 0.0f};
    s.target = initial_val;
    s.config = cfg;
    return s;
}

void spring2d_set_target(Spring2D* s, Vec2 target) {
    if (s) s->target = target;
}

void spring2d_snap(Spring2D* s, Vec2 val) {
    if (s) {
        s->pos = val;
        s->vel = (Vec2){0.0f, 0.0f};
        s->target = val;
    }
}

void spring2d_update(Spring2D* s, float dt) {
    if (!s || dt <= 0.0f) return;
    spring1d_integrate(&s->pos.x, &s->vel.x, s->target.x, s->config, dt);
    spring1d_integrate(&s->pos.y, &s->vel.y, s->target.y, s->config, dt);
}

bool spring2d_is_settled(const Spring2D* s, float eps_pos, float eps_vel) {
    if (!s) return true;
    return (fabsf(s->pos.x - s->target.x) <= eps_pos) &&
           (fabsf(s->pos.y - s->target.y) <= eps_pos) &&
           (fabsf(s->vel.x) <= eps_vel) &&
           (fabsf(s->vel.y) <= eps_vel);
}

// -----------------------------------------------------------------------------
// Spring Rect
// -----------------------------------------------------------------------------

SpringRect spring_rect_make(Rect initial_val, SpringConfig cfg) {
    SpringRect s;
    s.pos = initial_val;
    s.vel = (Rect){0.0f, 0.0f, 0.0f, 0.0f};
    s.target = initial_val;
    s.config = cfg;
    return s;
}

void spring_rect_set_target(SpringRect* s, Rect target) {
    if (s) s->target = target;
}

void spring_rect_snap(SpringRect* s, Rect val) {
    if (s) {
        s->pos = val;
        s->vel = (Rect){0.0f, 0.0f, 0.0f, 0.0f};
        s->target = val;
    }
}

void spring_rect_update(SpringRect* s, float dt) {
    if (!s || dt <= 0.0f) return;
    spring1d_integrate(&s->pos.x, &s->vel.x, s->target.x, s->config, dt);
    spring1d_integrate(&s->pos.y, &s->vel.y, s->target.y, s->config, dt);
    spring1d_integrate(&s->pos.w, &s->vel.w, s->target.w, s->config, dt);
    spring1d_integrate(&s->pos.h, &s->vel.h, s->target.h, s->config, dt);
}

bool spring_rect_is_settled(const SpringRect* s, float eps_pos, float eps_vel) {
    if (!s) return true;
    return (fabsf(s->pos.x - s->target.x) <= eps_pos) &&
           (fabsf(s->pos.y - s->target.y) <= eps_pos) &&
           (fabsf(s->pos.w - s->target.w) <= eps_pos) &&
           (fabsf(s->pos.h - s->target.h) <= eps_pos) &&
           (fabsf(s->vel.x) <= eps_vel) &&
           (fabsf(s->vel.y) <= eps_vel) &&
           (fabsf(s->vel.w) <= eps_vel) &&
           (fabsf(s->vel.h) <= eps_vel);
}

// -----------------------------------------------------------------------------
// Spring Color
// -----------------------------------------------------------------------------

SpringColor spring_color_make(Color initial_val, SpringConfig cfg) {
    SpringColor s;
    s.pos = initial_val;
    s.vel = (Color){0.0f, 0.0f, 0.0f, 0.0f};
    s.target = initial_val;
    s.config = cfg;
    return s;
}

void spring_color_set_target(SpringColor* s, Color target) {
    if (s) s->target = target;
}

void spring_color_snap(SpringColor* s, Color val) {
    if (s) {
        s->pos = val;
        s->vel = (Color){0.0f, 0.0f, 0.0f, 0.0f};
        s->target = val;
    }
}

void spring_color_update(SpringColor* s, float dt) {
    if (!s || dt <= 0.0f) return;
    spring1d_integrate(&s->pos.r, &s->vel.r, s->target.r, s->config, dt);
    spring1d_integrate(&s->pos.g, &s->vel.g, s->target.g, s->config, dt);
    spring1d_integrate(&s->pos.b, &s->vel.b, s->target.b, s->config, dt);
    spring1d_integrate(&s->pos.a, &s->vel.a, s->target.a, s->config, dt);
}

bool spring_color_is_settled(const SpringColor* s, float eps) {
    if (!s) return true;
    return (fabsf(s->pos.r - s->target.r) <= eps) &&
           (fabsf(s->pos.g - s->target.g) <= eps) &&
           (fabsf(s->pos.b - s->target.b) <= eps) &&
           (fabsf(s->pos.a - s->target.a) <= eps);
}

// -----------------------------------------------------------------------------
// File Pilot Gliding Caret Spring
// -----------------------------------------------------------------------------

CaretSpring caret_spring_make(float initial_x, float initial_y, float height) {
    CaretSpring c;
    SpringConfig lead_cfg  = (SpringConfig){.f = 24.0f, .zeta = 0.95f, .r = 0.5f};
    SpringConfig trail_cfg = (SpringConfig){.f = 18.0f, .zeta = 0.90f, .r = 0.0f};
    SpringConfig y_cfg     = (SpringConfig){.f = 22.0f, .zeta = 1.00f, .r = 0.0f};

    c.lead_x = spring1d_make(initial_x, lead_cfg);
    c.trail_x = spring1d_make(initial_x, trail_cfg);
    c.pos_y = spring1d_make(initial_y, y_cfg);
    c.height = spring1d_make(height, y_cfg);
    c.blink_time = 0.0f;
    c.is_typing = false;
    return c;
}

void caret_spring_set_target(CaretSpring* c, float target_x, float target_y, float height) {
    if (!c) return;

    if (target_x > c->lead_x.target) {
        // Moving right: lead responds faster
        c->lead_x.config.f = 24.0f;
        c->trail_x.config.f = 18.0f;
    } else if (target_x < c->lead_x.target) {
        // Moving left: trail responds faster
        c->trail_x.config.f = 24.0f;
        c->lead_x.config.f = 18.0f;
    }

    spring1d_set_target(&c->lead_x, target_x);
    spring1d_set_target(&c->trail_x, target_x);
    spring1d_set_target(&c->pos_y, target_y);
    spring1d_set_target(&c->height, height);
    c->blink_time = 0.0f; // Reset blink cycle on motion
}

void caret_spring_snap(CaretSpring* c, float target_x, float target_y, float height) {
    if (!c) return;
    spring1d_snap(&c->lead_x, target_x);
    spring1d_snap(&c->trail_x, target_x);
    spring1d_snap(&c->pos_y, target_y);
    spring1d_snap(&c->height, height);
    c->blink_time = 0.0f;
}

void caret_spring_notify_typing(CaretSpring* c) {
    if (!c) return;
    c->blink_time = 0.0f; // Keep solid while typing
}

void caret_spring_update(CaretSpring* c, float dt) {
    if (!c || dt <= 0.0f) return;
    spring1d_update(&c->lead_x, dt);
    spring1d_update(&c->trail_x, dt);
    spring1d_update(&c->pos_y, dt);
    spring1d_update(&c->height, dt);
    c->blink_time += dt;
}

Rect caret_spring_get_bounds(const CaretSpring* c, float min_caret_width) {
    if (!c) return (Rect){0, 0, 0, 0};

    float x0 = fminf(c->lead_x.pos, c->trail_x.pos);
    float x1 = fmaxf(c->lead_x.pos, c->trail_x.pos);

    float w = x1 - x0 + min_caret_width;
    if (w < min_caret_width) w = min_caret_width;

    return rect_make(x0, c->pos_y.pos, w, c->height.pos);
}

float caret_spring_get_alpha(const CaretSpring* c) {
    if (!c) return 1.0f;

    // Solid during motion
    bool in_motion = fabsf(c->lead_x.vel) > 1.0f || fabsf(c->trail_x.vel) > 1.0f;
    if (in_motion || c->blink_time < 0.5f) {
        return 1.0f;
    }

    // Smooth sinusoidal blink
    float phase = (c->blink_time - 0.5f) * 3.5f;
    float sin_val = sinf(phase);
    return (sin_val > 0.0f) ? 1.0f : 0.0f;
}

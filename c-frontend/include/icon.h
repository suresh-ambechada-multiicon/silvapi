#ifndef ICON_H
#define ICON_H

#include "silvapi.h"
#include "gl_backend.h"

void icon_system_init(void);
void icon_system_shutdown(void);
Rect icon_get_uv(IconType icon);
unsigned int icon_get_texture_id(void);
void icon_draw(IconType icon, Rect rect, Color color);

#endif // ICON_H

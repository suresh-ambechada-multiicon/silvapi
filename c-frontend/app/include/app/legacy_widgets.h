#ifndef APP_LEGACY_WIDGETS_H
#define APP_LEGACY_WIDGETS_H

#include "app/model.h"
#include "ui/ui.h"

// Immediate-mode widgets that take application model types. They live in app/
// so that libs/ui stays free of app headers. Each one is deleted as its callers
// move to the declarative components.
bool ui_kv_editor_legacy(const char* id_str, KeyValue** items_ptr, size_t* count_ptr,
                         Rect rect, float* scroll_y);



#endif // APP_LEGACY_WIDGETS_H

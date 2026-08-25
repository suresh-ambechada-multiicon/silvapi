#ifndef STORAGE_H
#define STORAGE_H

#include "base/base.h"
#include "app/model.h"

typedef struct {
    char id[64];
    char request_id[64];
    char timestamp[64];
    char method[16];
    int status_code;
    uint64_t latency_ms;
    size_t size_bytes;
    HttpResponse response;
} HistoryEntry;

bool storage_init(void);
void storage_shutdown(void);

Workspace* storage_load_workspace(void);
bool storage_save_workspace(const Workspace* ws);

bool storage_save_setting(const char* key, const char* value);
char* storage_load_setting(const char* key);

bool storage_add_response_history(const char* request_id, const HttpResponse* resp);
HistoryEntry* storage_load_response_history(const char* request_id, size_t* count);
void storage_free_history_entries(HistoryEntry* entries, size_t count);
bool storage_delete_response_history(const char* request_id);

#endif // STORAGE_H

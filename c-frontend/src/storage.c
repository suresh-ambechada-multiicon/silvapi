#include "storage.h"
#include <sqlite3.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

static sqlite3* s_db = NULL;
static char s_db_path[1024];

static void ensure_dir(const char* path) {
    char temp[1024];
    char* p = NULL;
    size_t len = strlen(path);
    if (len >= sizeof(temp)) return;
    strcpy(temp, path);
    for (p = temp + 1; *p; p++) {
        if (*p == '/') {
            *p = 0;
            mkdir(temp, 0755);
            *p = '/';
        }
    }
    mkdir(temp, 0755);
}

static Workspace* create_seed_workspace(void) {
    Workspace* ws = workspace_create("My Workspace");

    // Sample API Collection (matching screenshot 0 & 1)
    Collection* sample = collection_create("Sample API");
    
    // Users Folder
    Folder* users_folder = folder_create("Users");
    
    // 1. GET List users
    ApiRequest* req_list = api_request_create("List users", METHOD_GET, "https://jsonplaceholder.typicode.com/users");
    CollectionItem* item1 = calloc(1, sizeof(CollectionItem));
    item1->type = ITEM_REQUEST;
    item1->request = req_list;
    users_folder->items = malloc(sizeof(CollectionItem*) * 3);
    users_folder->items[0] = item1;

    // 2. GET Get user
    ApiRequest* req_get = api_request_create("Get user", METHOD_GET, "https://jsonplaceholder.typicode.com/users/1");
    CollectionItem* item2 = calloc(1, sizeof(CollectionItem));
    item2->type = ITEM_REQUEST;
    item2->request = req_get;
    users_folder->items[1] = item2;

    // 3. POST Create user
    ApiRequest* req_post = api_request_create("Create user", METHOD_POST, "https://jsonplaceholder.typicode.com/users");
    req_post->body.body_type = BODY_JSON;
    free(req_post->body.content);
    req_post->body.content = strdup("{\n  \"name\": \"John Doe\",\n  \"email\": \"john@example.com\"\n}");
    kv_array_add(&req_post->headers, &req_post->headers_count, "Content-Type", "application/json", true);
    kv_array_add(&req_post->headers, &req_post->headers_count, "Accept", "application/json", true);
    CollectionItem* item3 = calloc(1, sizeof(CollectionItem));
    item3->type = ITEM_REQUEST;
    item3->request = req_post;
    users_folder->items[2] = item3;
    users_folder->items_count = 3;

    // 4. GET Health check (under collection root)
    ApiRequest* req_health = api_request_create("Health check", METHOD_GET, "https://jsonplaceholder.typicode.com/todos/1");
    CollectionItem* item_health = calloc(1, sizeof(CollectionItem));
    item_health->type = ITEM_REQUEST;
    item_health->request = req_health;

    // 5. GET New Request
    ApiRequest* req_new = api_request_create("New Request", METHOD_GET, "https://jsonplaceholder.typicode.com/posts/1");
    CollectionItem* item_new = calloc(1, sizeof(CollectionItem));
    item_new->type = ITEM_REQUEST;
    item_new->request = req_new;

    // Add Folder + Items to Collection
    CollectionItem* folder_item = calloc(1, sizeof(CollectionItem));
    folder_item->type = ITEM_FOLDER;
    folder_item->folder = users_folder;

    sample->items = malloc(sizeof(CollectionItem*) * 3);
    sample->items[0] = folder_item;
    sample->items[1] = item_health;
    sample->items[2] = item_new;
    sample->items_count = 3;

    ws->collections = malloc(sizeof(Collection*) * 1);
    ws->collections[0] = sample;
    ws->collections_count = 1;

    // Default variable
    kv_array_add(&ws->variables, &ws->variables_count, "baseUrl", "https://jsonplaceholder.typicode.com", true);

    return ws;
}

bool storage_init(void) {
    const char* xdg_data = getenv("XDG_DATA_HOME");
    char dir_path[1024];
    if (xdg_data && *xdg_data) {
        snprintf(dir_path, sizeof(dir_path), "%s/silvapi", xdg_data);
    } else {
        const char* home = getenv("HOME");
        snprintf(dir_path, sizeof(dir_path), "%s/.local/share/silvapi", home ? home : "/tmp");
    }
    ensure_dir(dir_path);
    snprintf(s_db_path, sizeof(s_db_path), "%s/silvapi.sqlite", dir_path);

    int rc = sqlite3_open(s_db_path, &s_db);
    if (rc != SQLITE_OK) {
        fprintf(stderr, "Cannot open SQLite database: %s\n", sqlite3_errmsg(s_db));
        return false;
    }

    // Enable WAL mode for high performance concurrency
    sqlite3_exec(s_db, "PRAGMA journal_mode=WAL;", NULL, NULL, NULL);

    // Create tables
    const char* schema =
        "CREATE TABLE IF NOT EXISTS workspace ("
        "  id INTEGER PRIMARY KEY,"
        "  json TEXT NOT NULL,"
        "  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP"
        ");"
        "CREATE TABLE IF NOT EXISTS settings ("
        "  key TEXT PRIMARY KEY,"
        "  value TEXT NOT NULL,"
        "  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP"
        ");"
        "CREATE TABLE IF NOT EXISTS response_history ("
        "  id TEXT PRIMARY KEY,"
        "  request_id TEXT NOT NULL,"
        "  json TEXT NOT NULL,"
        "  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP"
        ");";

    char* err_msg = NULL;
    rc = sqlite3_exec(s_db, schema, NULL, NULL, &err_msg);
    if (rc != SQLITE_OK) {
        fprintf(stderr, "SQLite schema error: %s\n", err_msg);
        sqlite3_free(err_msg);
        return false;
    }

    return true;
}

void storage_shutdown(void) {
    if (s_db) {
        sqlite3_close(s_db);
        s_db = NULL;
    }
}

Workspace* storage_load_workspace(void) {
    if (!s_db) return create_seed_workspace();

    sqlite3_stmt* stmt = NULL;
    const char* sql = "SELECT json FROM workspace WHERE id = 1 LIMIT 1;";
    if (sqlite3_prepare_v2(s_db, sql, -1, &stmt, NULL) != SQLITE_OK) {
        return create_seed_workspace();
    }

    Workspace* ws = NULL;
    if (sqlite3_step(stmt) == SQLITE_ROW) {
        const char* json_str = (const char*)sqlite3_column_text(stmt, 0);
        if (json_str) {
            cJSON* root = cJSON_Parse(json_str);
            if (root) {
                ws = workspace_from_json(root);
                cJSON_Delete(root);
            }
        }
    }
    sqlite3_finalize(stmt);

    if (!ws) {
        ws = create_seed_workspace();
        storage_save_workspace(ws);
    }
    return ws;
}

bool storage_save_workspace(const Workspace* ws) {
    if (!s_db || !ws) return false;

    cJSON* root = workspace_to_json(ws);
    if (!root) return false;
    char* json_str = cJSON_PrintUnformatted(root);
    cJSON_Delete(root);
    if (!json_str) return false;

    sqlite3_stmt* stmt = NULL;
    const char* sql = "INSERT INTO workspace (id, json, updated_at) VALUES (1, ?1, CURRENT_TIMESTAMP) "
                      "ON CONFLICT(id) DO UPDATE SET json = ?1, updated_at = CURRENT_TIMESTAMP;";
    
    if (sqlite3_prepare_v2(s_db, sql, -1, &stmt, NULL) != SQLITE_OK) {
        free(json_str);
        return false;
    }

    sqlite3_bind_text(stmt, 1, json_str, -1, SQLITE_STATIC);
    int rc = sqlite3_step(stmt);
    sqlite3_finalize(stmt);
    free(json_str);

    return (rc == SQLITE_DONE);
}

bool storage_save_setting(const char* key, const char* value) {
    if (!s_db || !key || !value) return false;
    sqlite3_stmt* stmt = NULL;
    const char* sql = "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP) "
                      "ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = CURRENT_TIMESTAMP;";
    if (sqlite3_prepare_v2(s_db, sql, -1, &stmt, NULL) != SQLITE_OK) return false;
    sqlite3_bind_text(stmt, 1, key, -1, SQLITE_STATIC);
    sqlite3_bind_text(stmt, 2, value, -1, SQLITE_STATIC);
    int rc = sqlite3_step(stmt);
    sqlite3_finalize(stmt);
    return (rc == SQLITE_DONE);
}

char* storage_load_setting(const char* key) {
    if (!s_db || !key) return NULL;
    sqlite3_stmt* stmt = NULL;
    const char* sql = "SELECT value FROM settings WHERE key = ?1 LIMIT 1;";
    if (sqlite3_prepare_v2(s_db, sql, -1, &stmt, NULL) != SQLITE_OK) return NULL;
    sqlite3_bind_text(stmt, 1, key, -1, SQLITE_STATIC);

    char* result = NULL;
    if (sqlite3_step(stmt) == SQLITE_ROW) {
        const char* val = (const char*)sqlite3_column_text(stmt, 0);
        if (val) result = strdup(val);
    }
    sqlite3_finalize(stmt);
    return result;
}

bool storage_add_response_history(const char* request_id, const HttpResponse* resp) {
    if (!s_db || !request_id || !resp) return false;

    cJSON* root = cJSON_CreateObject();
    cJSON_AddStringToObject(root, "request_id", request_id);
    cJSON_AddNumberToObject(root, "status_code", resp->status_code);
    cJSON_AddStringToObject(root, "status_text", resp->status_text);
    cJSON_AddNumberToObject(root, "latency_ms", (double)resp->latency_ms);
    cJSON_AddNumberToObject(root, "size_bytes", (double)resp->body_size);
    cJSON_AddStringToObject(root, "timestamp", resp->timestamp);
    cJSON_AddStringToObject(root, "body", resp->body ? resp->body : "");
    if (resp->error) cJSON_AddStringToObject(root, "error", resp->error);

    cJSON* headers = cJSON_CreateArray();
    for (size_t i = 0; i < resp->headers_count; i++) {
        cJSON* h = cJSON_CreateObject();
        cJSON_AddStringToObject(h, "key", resp->headers[i].key);
        cJSON_AddStringToObject(h, "value", resp->headers[i].value);
        cJSON_AddItemToArray(headers, h);
    }
    cJSON_AddItemToObject(root, "headers", headers);

    char* json_str = cJSON_PrintUnformatted(root);
    cJSON_Delete(root);
    if (!json_str) return false;

    char history_id[64];
    generate_uuid(history_id, sizeof(history_id));

    sqlite3_stmt* stmt = NULL;
    const char* sql = "INSERT INTO response_history (id, request_id, json, created_at) VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP);";
    if (sqlite3_prepare_v2(s_db, sql, -1, &stmt, NULL) != SQLITE_OK) {
        free(json_str);
        return false;
    }

    sqlite3_bind_text(stmt, 1, history_id, -1, SQLITE_STATIC);
    sqlite3_bind_text(stmt, 2, request_id, -1, SQLITE_STATIC);
    sqlite3_bind_text(stmt, 3, json_str, -1, SQLITE_STATIC);
    int rc = sqlite3_step(stmt);
    sqlite3_finalize(stmt);
    free(json_str);

    return (rc == SQLITE_DONE);
}

HistoryEntry* storage_load_response_history(const char* request_id, size_t* out_count) {
    if (!s_db || !request_id || !out_count) return NULL;
    *out_count = 0;

    sqlite3_stmt* stmt = NULL;
    const char* sql = "SELECT id, json, created_at FROM response_history WHERE request_id = ?1 ORDER BY created_at DESC LIMIT 20;";
    if (sqlite3_prepare_v2(s_db, sql, -1, &stmt, NULL) != SQLITE_OK) return NULL;
    sqlite3_bind_text(stmt, 1, request_id, -1, SQLITE_STATIC);

    size_t cap = 8;
    HistoryEntry* entries = malloc(sizeof(HistoryEntry) * cap);
    size_t count = 0;

    while (sqlite3_step(stmt) == SQLITE_ROW) {
        if (count >= cap) {
            cap *= 2;
            entries = realloc(entries, sizeof(HistoryEntry) * cap);
        }
        HistoryEntry* entry = &entries[count];
        memset(entry, 0, sizeof(HistoryEntry));

        const char* hid = (const char*)sqlite3_column_text(stmt, 0);
        const char* json_str = (const char*)sqlite3_column_text(stmt, 1);
        const char* created_at = (const char*)sqlite3_column_text(stmt, 2);

        if (hid) strncpy(entry->id, hid, sizeof(entry->id) - 1);
        strncpy(entry->request_id, request_id, sizeof(entry->request_id) - 1);
        if (created_at) strncpy(entry->timestamp, created_at, sizeof(entry->timestamp) - 1);

        if (json_str) {
            cJSON* obj = cJSON_Parse(json_str);
            if (obj) {
                cJSON* item;
                if ((item = cJSON_GetObjectItem(obj, "status_code"))) entry->status_code = item->valueint;
                if ((item = cJSON_GetObjectItem(obj, "status_text")) && item->valuestring) strncpy(entry->response.status_text, item->valuestring, sizeof(entry->response.status_text) - 1);
                if ((item = cJSON_GetObjectItem(obj, "latency_ms"))) entry->latency_ms = (uint64_t)item->valuedouble;
                if ((item = cJSON_GetObjectItem(obj, "size_bytes"))) entry->size_bytes = (size_t)item->valuedouble;
                if ((item = cJSON_GetObjectItem(obj, "body")) && item->valuestring) entry->response.body = strdup(item->valuestring);
                if ((item = cJSON_GetObjectItem(obj, "error")) && item->valuestring) entry->response.error = strdup(item->valuestring);

                entry->response.status_code = entry->status_code;
                entry->response.latency_ms = entry->latency_ms;
                entry->response.body_size = entry->size_bytes;
                strncpy(entry->response.timestamp, entry->timestamp, sizeof(entry->response.timestamp) - 1);

                cJSON* headers = cJSON_GetObjectItem(obj, "headers");
                if (headers && cJSON_IsArray(headers)) {
                    int h_sz = cJSON_GetArraySize(headers);
                    entry->response.headers = malloc(sizeof(KeyValue) * h_sz);
                    entry->response.headers_count = h_sz;
                    for (int i = 0; i < h_sz; i++) {
                        cJSON* h = cJSON_GetArrayItem(headers, i);
                        cJSON* hk = cJSON_GetObjectItem(h, "key");
                        cJSON* hv = cJSON_GetObjectItem(h, "value");
                        strncpy(entry->response.headers[i].key, hk && hk->valuestring ? hk->valuestring : "", sizeof(entry->response.headers[i].key) - 1);
                        strncpy(entry->response.headers[i].value, hv && hv->valuestring ? hv->valuestring : "", sizeof(entry->response.headers[i].value) - 1);
                        entry->response.headers[i].enabled = true;
                    }
                }
                cJSON_Delete(obj);
            }
        }
        count++;
    }
    sqlite3_finalize(stmt);

    *out_count = count;
    return entries;
}

void storage_free_history_entries(HistoryEntry* entries, size_t count) {
    if (!entries) return;
    for (size_t i = 0; i < count; i++) {
        http_response_free_contents(&entries[i].response);
    }
    free(entries);
}

bool storage_delete_response_history(const char* request_id) {
    if (!s_db || !request_id) return false;
    sqlite3_stmt* stmt = NULL;
    const char* sql = "DELETE FROM response_history WHERE request_id = ?1;";
    if (sqlite3_prepare_v2(s_db, sql, -1, &stmt, NULL) != SQLITE_OK) return false;
    sqlite3_bind_text(stmt, 1, request_id, -1, SQLITE_STATIC);
    int rc = sqlite3_step(stmt);
    sqlite3_finalize(stmt);
    return (rc == SQLITE_DONE);
}

#ifndef MODEL_H
#define MODEL_H

#include "base/base.h"
#include "cJSON.h"

typedef enum {
    METHOD_GET = 0,
    METHOD_POST,
    METHOD_PUT,
    METHOD_PATCH,
    METHOD_DELETE,
    METHOD_HEAD,
    METHOD_OPTIONS,
    METHOD_COUNT
} HttpMethod;

const char* http_method_to_string(HttpMethod method);
HttpMethod http_method_from_string(const char* str);

typedef enum {
    BODY_NO_BODY = 0,
    BODY_JSON,
    BODY_FORM_DATA,
    BODY_URLENCODED,
    BODY_RAW,
    BODY_BINARY,
    BODY_COUNT
} BodyType;

const char* body_type_to_string(BodyType type);
BodyType body_type_from_string(const char* str);

typedef enum {
    AUTH_NONE = 0,
    AUTH_BEARER,
    AUTH_BASIC,
    AUTH_API_KEY,
    AUTH_AWS_V4,
    AUTH_JWT,
    AUTH_OAUTH2,
    AUTH_COUNT
} AuthType;

const char* auth_type_to_string(AuthType type);
AuthType auth_type_from_string(const char* str);

typedef struct {
    char id[64];
    char key[256];
    char value[1024];
    bool enabled;
    char description[256];
} KeyValue;

typedef struct {
    char id[64];
    char name[256];
    char value[1024];
    bool is_file;
    char file_path[1024];
    char content_type[128];
    bool enabled;
} FormDataPart;

typedef struct {
    AuthType type;
    char bearer_token[1024];
    char bearer_prefix[128];
    char basic_username[256];
    char basic_password[256];
    char api_key_name[256];
    char api_key_value[1024];
    char api_key_location[32]; // "header" or "query"
    char aws_access_key_id[256];
    char aws_secret_access_key[256];
    char aws_service[128];
    char aws_region[128];
    char aws_session_token[1024];
    char jwt_secret[1024];
    char jwt_payload[4096];
    char oauth_client_id[256];
    char oauth_client_secret[256];
    char oauth_auth_url[1024];
    char oauth_token_url[1024];
    char oauth_scope[512];
    char oauth_redirect_uri[512];
} AuthConfig;

typedef struct {
    BodyType body_type;
    char* content; // dynamically allocated text/json/raw
    size_t content_len;
    size_t content_cap;
    KeyValue* urlencoded;
    size_t urlencoded_count;
    FormDataPart* form_data;
    size_t form_data_count;
    char binary_file_path[1024];
} RequestBody;

typedef struct {
    char id[64];
    char name[256];
    HttpMethod method;
    char url[2048];
    KeyValue* params;
    size_t params_count;
    KeyValue* headers;
    size_t headers_count;
    RequestBody body;
    AuthConfig auth;
    char description[2048];
    int sort_order;
    // Execution metadata
    int last_status;
    uint64_t last_duration_ms;
} ApiRequest;

typedef struct CollectionItem CollectionItem;

typedef struct {
    char id[64];
    char name[256];
    char description[1024];
    KeyValue* headers;
    size_t headers_count;
    KeyValue* variables;
    size_t variables_count;
    AuthConfig auth;
    CollectionItem** items;
    size_t items_count;
    bool is_expanded;
} Folder;

typedef enum {
    ITEM_REQUEST = 0,
    ITEM_FOLDER
} ItemType;

struct CollectionItem {
    ItemType type;
    union {
        ApiRequest* request;
        Folder* folder;
    };
};

typedef struct {
    char id[64];
    char name[256];
    char description[1024];
    KeyValue* headers;
    size_t headers_count;
    KeyValue* variables;
    size_t variables_count;
    AuthConfig auth;
    CollectionItem** items;
    size_t items_count;
    bool is_expanded;
} Collection;

typedef struct {
    char id[64];
    char name[256];
    Collection** collections;
    size_t collections_count;
    KeyValue* variables;
    size_t variables_count;
} Workspace;

typedef struct {
    char name[64];
    double duration_ms;
    double start_offset_ms;
    char description[256];
} TimelineEvent;

typedef struct {
    char request_id[64];
    int status_code;
    char status_text[64];
    KeyValue* headers;
    size_t headers_count;
    char* body;
    size_t body_size;
    uint64_t latency_ms;
    char timestamp[64];
    char* error;
    bool is_loading;
    TimelineEvent timeline[16];
    size_t timeline_count;
} HttpResponse;

typedef struct {
    char request_id[64];
    char title[256];
    HttpMethod method;
    bool is_dirty;
} TabItem;

// Allocators and memory management
ApiRequest* api_request_create(const char* name, HttpMethod method, const char* url);
void api_request_free(ApiRequest* req);
ApiRequest* api_request_clone(const ApiRequest* src);

Folder* folder_create(const char* name);
void folder_free(Folder* f);

Collection* collection_create(const char* name);
void collection_free(Collection* col);

Workspace* workspace_create(const char* name);
void workspace_free(Workspace* ws);

void http_response_free_contents(HttpResponse* resp);

// KeyValue helpers
void kv_array_add(KeyValue** array, size_t* count, const char* key, const char* value, bool enabled);
void kv_array_remove(KeyValue** array, size_t* count, size_t index);
void kv_array_free(KeyValue* array, size_t count);

// Variable Interpolation (e.g. {{baseUrl}}/users, {{$guid}}, {{$timestamp}})
char* interpolate_variables(const char* input, const Workspace* ws, const Collection* col, const Folder* folder);

// cJSON Serialization / Deserialization
cJSON* workspace_to_json(const Workspace* ws);
Workspace* workspace_from_json(const cJSON* json);

cJSON* api_request_to_json(const ApiRequest* req);
ApiRequest* api_request_from_json(const cJSON* json);

cJSON* http_response_to_json(const HttpResponse* resp);
HttpResponse* http_response_from_json(const cJSON* json);

// Helper to generate UUIDv4 string
void generate_uuid(char* out_buf, size_t buf_size);

#endif // MODEL_H

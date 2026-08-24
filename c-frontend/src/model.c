#include "model.h"
#include <ctype.h>
#include <strings.h>

void generate_uuid(char* out_buf, size_t buf_size) {
    if (!out_buf || buf_size < 37) return;
    static bool seeded = false;
    if (!seeded) {
        srand((unsigned int)time(NULL));
        seeded = true;
    }
    const char* hex = "0123456789abcdef";
    for (int i = 0; i < 36; i++) {
        if (i == 8 || i == 13 || i == 18 || i == 23) {
            out_buf[i] = '-';
        } else if (i == 14) {
            out_buf[i] = '4'; // UUID v4
        } else if (i == 19) {
            out_buf[i] = hex[(rand() % 4) + 8]; // variant 1
        } else {
            out_buf[i] = hex[rand() % 16];
        }
    }
    out_buf[36] = '\0';
}

const char* http_method_to_string(HttpMethod method) {
    switch (method) {
        case METHOD_GET: return "GET";
        case METHOD_POST: return "POST";
        case METHOD_PUT: return "PUT";
        case METHOD_PATCH: return "PATCH";
        case METHOD_DELETE: return "DELETE";
        case METHOD_HEAD: return "HEAD";
        case METHOD_OPTIONS: return "OPTIONS";
        default: return "GET";
    }
}

HttpMethod http_method_from_string(const char* str) {
    if (!str) return METHOD_GET;
    if (strcasecmp(str, "GET") == 0) return METHOD_GET;
    if (strcasecmp(str, "POST") == 0) return METHOD_POST;
    if (strcasecmp(str, "PUT") == 0) return METHOD_PUT;
    if (strcasecmp(str, "PATCH") == 0) return METHOD_PATCH;
    if (strcasecmp(str, "DELETE") == 0) return METHOD_DELETE;
    if (strcasecmp(str, "HEAD") == 0) return METHOD_HEAD;
    if (strcasecmp(str, "OPTIONS") == 0) return METHOD_OPTIONS;
    return METHOD_GET;
}

const char* body_type_to_string(BodyType type) {
    switch (type) {
        case BODY_NO_BODY: return "NoBody";
        case BODY_JSON: return "Json";
        case BODY_FORM_DATA: return "FormData";
        case BODY_URLENCODED: return "UrlEncoded";
        case BODY_RAW: return "Raw";
        case BODY_BINARY: return "BinaryFile";
        default: return "NoBody";
    }
}


BodyType body_type_from_string(const char* str) {
    if (!str) return BODY_NO_BODY;
    if (strcasecmp(str, "NoBody") == 0 || strcasecmp(str, "none") == 0) return BODY_NO_BODY;
    if (strcasecmp(str, "Json") == 0 || strcasecmp(str, "json") == 0) return BODY_JSON;
    if (strcasecmp(str, "FormData") == 0 || strcasecmp(str, "form_data") == 0) return BODY_FORM_DATA;
    if (strcasecmp(str, "UrlEncoded") == 0 || strcasecmp(str, "urlencoded") == 0) return BODY_URLENCODED;
    if (strcasecmp(str, "Raw") == 0 || strcasecmp(str, "raw") == 0) return BODY_RAW;
    if (strcasecmp(str, "BinaryFile") == 0 || strcasecmp(str, "binary") == 0) return BODY_BINARY;
    return BODY_NO_BODY;
}

const char* auth_type_to_string(AuthType type) {
    switch (type) {
        case AUTH_NONE: return "None";
        case AUTH_BEARER: return "Bearer";
        case AUTH_BASIC: return "Basic";
        case AUTH_API_KEY: return "ApiKey";
        case AUTH_AWS_V4: return "AwsV4";
        case AUTH_JWT: return "Jwt";
        case AUTH_OAUTH2: return "OAuth2";
        default: return "None";
    }
}

AuthType auth_type_from_string(const char* str) {
    if (!str) return AUTH_NONE;
    if (strcasecmp(str, "None") == 0 || strcasecmp(str, "none") == 0) return AUTH_NONE;
    if (strcasecmp(str, "Bearer") == 0 || strcasecmp(str, "bearer") == 0) return AUTH_BEARER;
    if (strcasecmp(str, "Basic") == 0 || strcasecmp(str, "basic") == 0) return AUTH_BASIC;
    if (strcasecmp(str, "ApiKey") == 0 || strcasecmp(str, "api_key") == 0) return AUTH_API_KEY;
    if (strcasecmp(str, "AwsV4") == 0 || strcasecmp(str, "aws_v4") == 0) return AUTH_AWS_V4;
    if (strcasecmp(str, "Jwt") == 0 || strcasecmp(str, "jwt") == 0) return AUTH_JWT;
    if (strcasecmp(str, "OAuth2") == 0 || strcasecmp(str, "oauth2") == 0) return AUTH_OAUTH2;
    return AUTH_NONE;
}

void kv_array_add(KeyValue** array, size_t* count, const char* key, const char* value, bool enabled) {
    if (!array || !count) return;
    size_t old_count = *count;
    *array = realloc(*array, sizeof(KeyValue) * (old_count + 1));
    KeyValue* item = &(*array)[old_count];
    generate_uuid(item->id, sizeof(item->id));
    strncpy(item->key, key ? key : "", sizeof(item->key) - 1);
    item->key[sizeof(item->key) - 1] = '\0';
    strncpy(item->value, value ? value : "", sizeof(item->value) - 1);
    item->value[sizeof(item->value) - 1] = '\0';
    item->enabled = enabled;
    item->description[0] = '\0';
    *count = old_count + 1;
}

void kv_array_remove(KeyValue** array, size_t* count, size_t index) {
    if (!array || !count || index >= *count) return;
    for (size_t i = index; i < *count - 1; i++) {
        (*array)[i] = (*array)[i + 1];
    }
    *count = *count - 1;
    if (*count == 0) {
        free(*array);
        *array = NULL;
    } else {
        *array = realloc(*array, sizeof(KeyValue) * (*count));
    }
}

void kv_array_free(KeyValue* array, size_t count) {
    (void)count;
    if (array) free(array);
}

ApiRequest* api_request_create(const char* name, HttpMethod method, const char* url) {
    ApiRequest* req = calloc(1, sizeof(ApiRequest));
    generate_uuid(req->id, sizeof(req->id));
    strncpy(req->name, name ? name : "New Request", sizeof(req->name) - 1);
    req->method = method;
    strncpy(req->url, url ? url : "", sizeof(req->url) - 1);
    req->body.body_type = BODY_NO_BODY;
    req->body.content = strdup("");
    req->body.content_len = 0;
    req->body.content_cap = 1;
    req->auth.type = AUTH_NONE;
    return req;
}

void api_request_free(ApiRequest* req) {
    if (!req) return;
    if (req->params) free(req->params);
    if (req->headers) free(req->headers);
    if (req->body.content) free(req->body.content);
    if (req->body.urlencoded) free(req->body.urlencoded);
    if (req->body.form_data) free(req->body.form_data);
    free(req);
}

ApiRequest* api_request_clone(const ApiRequest* src) {
    if (!src) return NULL;
    ApiRequest* dst = calloc(1, sizeof(ApiRequest));
    memcpy(dst, src, sizeof(ApiRequest));
    
    // Deep copies
    if (src->params_count > 0 && src->params) {
        dst->params = malloc(sizeof(KeyValue) * src->params_count);
        memcpy(dst->params, src->params, sizeof(KeyValue) * src->params_count);
    } else {
        dst->params = NULL;
        dst->params_count = 0;
    }

    if (src->headers_count > 0 && src->headers) {
        dst->headers = malloc(sizeof(KeyValue) * src->headers_count);
        memcpy(dst->headers, src->headers, sizeof(KeyValue) * src->headers_count);
    } else {
        dst->headers = NULL;
        dst->headers_count = 0;
    }

    if (src->body.content) {
        dst->body.content = strdup(src->body.content);
    } else {
        dst->body.content = strdup("");
    }

    if (src->body.urlencoded_count > 0 && src->body.urlencoded) {
        dst->body.urlencoded = malloc(sizeof(KeyValue) * src->body.urlencoded_count);
        memcpy(dst->body.urlencoded, src->body.urlencoded, sizeof(KeyValue) * src->body.urlencoded_count);
    } else {
        dst->body.urlencoded = NULL;
        dst->body.urlencoded_count = 0;
    }

    if (src->body.form_data_count > 0 && src->body.form_data) {
        dst->body.form_data = malloc(sizeof(FormDataPart) * src->body.form_data_count);
        memcpy(dst->body.form_data, src->body.form_data, sizeof(FormDataPart) * src->body.form_data_count);
    } else {
        dst->body.form_data = NULL;
        dst->body.form_data_count = 0;
    }

    return dst;
}

Folder* folder_create(const char* name) {
    Folder* f = calloc(1, sizeof(Folder));
    generate_uuid(f->id, sizeof(f->id));
    strncpy(f->name, name ? name : "New Folder", sizeof(f->name) - 1);
    f->is_expanded = true;
    return f;
}

void folder_free(Folder* f) {
    if (!f) return;
    if (f->headers) free(f->headers);
    if (f->variables) free(f->variables);
    for (size_t i = 0; i < f->items_count; i++) {
        CollectionItem* item = f->items[i];
        if (item) {
            if (item->type == ITEM_REQUEST) api_request_free(item->request);
            else if (item->type == ITEM_FOLDER) folder_free(item->folder);
            free(item);
        }
    }
    if (f->items) free(f->items);
    free(f);
}

Collection* collection_create(const char* name) {
    Collection* col = calloc(1, sizeof(Collection));
    generate_uuid(col->id, sizeof(col->id));
    strncpy(col->name, name ? name : "New Collection", sizeof(col->name) - 1);
    col->is_expanded = true;
    return col;
}

void collection_free(Collection* col) {
    if (!col) return;
    if (col->headers) free(col->headers);
    if (col->variables) free(col->variables);
    for (size_t i = 0; i < col->items_count; i++) {
        CollectionItem* item = col->items[i];
        if (item) {
            if (item->type == ITEM_REQUEST) api_request_free(item->request);
            else if (item->type == ITEM_FOLDER) folder_free(item->folder);
            free(item);
        }
    }
    if (col->items) free(col->items);
    free(col);
}

Workspace* workspace_create(const char* name) {
    Workspace* ws = calloc(1, sizeof(Workspace));
    generate_uuid(ws->id, sizeof(ws->id));
    strncpy(ws->name, name ? name : "Default Workspace", sizeof(ws->name) - 1);
    return ws;
}

void workspace_free(Workspace* ws) {
    if (!ws) return;
    for (size_t i = 0; i < ws->collections_count; i++) {
        collection_free(ws->collections[i]);
    }
    if (ws->collections) free(ws->collections);
    if (ws->variables) free(ws->variables);
    free(ws);
}

void http_response_free_contents(HttpResponse* resp) {
    if (!resp) return;
    if (resp->headers) {
        free(resp->headers);
        resp->headers = NULL;
        resp->headers_count = 0;
    }
    if (resp->body) {
        free(resp->body);
        resp->body = NULL;
    }
    if (resp->error) {
        free(resp->error);
        resp->error = NULL;
    }
}

// Variable Interpolation
char* interpolate_variables(const char* input, const Workspace* ws, const Collection* col, const Folder* folder) {
    if (!input) return strdup("");
    
    // Dynamic buffer
    size_t out_cap = strlen(input) + 256;
    char* out = malloc(out_cap);
    size_t out_len = 0;
    out[0] = '\0';

    const char* p = input;
    while (*p) {
        if (p[0] == '{' && p[1] == '{') {
            const char* end = strstr(p + 2, "}}");
            if (end) {
                size_t var_name_len = end - (p + 2);
                char var_name[256];
                if (var_name_len < sizeof(var_name)) {
                    strncpy(var_name, p + 2, var_name_len);
                    var_name[var_name_len] = '\0';
                    
                    // Trim whitespace
                    char* start = var_name;
                    while (isspace((unsigned char)*start)) start++;
                    char* trim_end = start + strlen(start) - 1;
                    while (trim_end > start && isspace((unsigned char)*trim_end)) {
                        *trim_end = '\0';
                        trim_end--;
                    }

                    const char* resolved_val = NULL;
                    char dynamic_val[64] = {0};

                    // Dynamic built-in variables
                    if (strcasecmp(start, "$guid") == 0 || strcasecmp(start, "$uuid") == 0) {
                        generate_uuid(dynamic_val, sizeof(dynamic_val));
                        resolved_val = dynamic_val;
                    } else if (strcasecmp(start, "$timestamp") == 0) {
                        snprintf(dynamic_val, sizeof(dynamic_val), "%ld", (long)time(NULL));
                        resolved_val = dynamic_val;
                    } else if (strcasecmp(start, "$randomInt") == 0) {
                        snprintf(dynamic_val, sizeof(dynamic_val), "%d", rand() % 1000);
                        resolved_val = dynamic_val;
                    }

                    // Look in folder variables
                    if (!resolved_val && folder) {
                        for (size_t i = 0; i < folder->variables_count; i++) {
                            if (folder->variables[i].enabled && strcmp(folder->variables[i].key, start) == 0) {
                                resolved_val = folder->variables[i].value;
                                break;
                            }
                        }
                    }

                    // Look in collection variables
                    if (!resolved_val && col) {
                        for (size_t i = 0; i < col->variables_count; i++) {
                            if (col->variables[i].enabled && strcmp(col->variables[i].key, start) == 0) {
                                resolved_val = col->variables[i].value;
                                break;
                            }
                        }
                    }

                    // Look in workspace variables
                    if (!resolved_val && ws) {
                        for (size_t i = 0; i < ws->variables_count; i++) {
                            if (ws->variables[i].enabled && strcmp(ws->variables[i].key, start) == 0) {
                                resolved_val = ws->variables[i].value;
                                break;
                            }
                        }
                    }

                    if (resolved_val) {
                        size_t rlen = strlen(resolved_val);
                        if (out_len + rlen + 1 >= out_cap) {
                            out_cap = (out_len + rlen + 256) * 2;
                            out = realloc(out, out_cap);
                        }
                        strcpy(out + out_len, resolved_val);
                        out_len += rlen;
                        p = end + 2;
                        continue;
                    }
                }
            }
        }
        
        // Append normal char
        if (out_len + 2 >= out_cap) {
            out_cap = (out_cap + 256) * 2;
            out = realloc(out, out_cap);
        }
        out[out_len++] = *p++;
        out[out_len] = '\0';
    }

    return out;
}

// JSON Serialization implementations
static cJSON* kv_to_json(const KeyValue* kv) {
    cJSON* obj = cJSON_CreateObject();
    cJSON_AddStringToObject(obj, "id", kv->id);
    cJSON_AddStringToObject(obj, "key", kv->key);
    cJSON_AddStringToObject(obj, "value", kv->value);
    cJSON_AddBoolToObject(obj, "enabled", kv->enabled);
    cJSON_AddStringToObject(obj, "description", kv->description);
    return obj;
}

static KeyValue kv_from_json(const cJSON* obj) {
    KeyValue kv = {0};
    cJSON* item;
    if ((item = cJSON_GetObjectItem(obj, "id")) && item->valuestring) strncpy(kv.id, item->valuestring, sizeof(kv.id) - 1);
    if ((item = cJSON_GetObjectItem(obj, "key")) && item->valuestring) strncpy(kv.key, item->valuestring, sizeof(kv.key) - 1);
    if ((item = cJSON_GetObjectItem(obj, "value")) && item->valuestring) strncpy(kv.value, item->valuestring, sizeof(kv.value) - 1);
    if ((item = cJSON_GetObjectItem(obj, "enabled"))) kv.enabled = cJSON_IsTrue(item);
    else kv.enabled = true;
    if ((item = cJSON_GetObjectItem(obj, "description")) && item->valuestring) strncpy(kv.description, item->valuestring, sizeof(kv.description) - 1);
    return kv;
}

cJSON* api_request_to_json(const ApiRequest* req) {
    if (!req) return NULL;
    cJSON* obj = cJSON_CreateObject();
    cJSON_AddStringToObject(obj, "id", req->id);
    cJSON_AddStringToObject(obj, "name", req->name);
    cJSON_AddStringToObject(obj, "method", http_method_to_string(req->method));
    cJSON_AddStringToObject(obj, "url", req->url);
    cJSON_AddStringToObject(obj, "description", req->description);
    cJSON_AddNumberToObject(obj, "sort_order", req->sort_order);

    // Params
    cJSON* params = cJSON_CreateArray();
    for (size_t i = 0; i < req->params_count; i++) {
        cJSON_AddItemToArray(params, kv_to_json(&req->params[i]));
    }
    cJSON_AddItemToObject(obj, "params", params);

    // Headers
    cJSON* headers = cJSON_CreateArray();
    for (size_t i = 0; i < req->headers_count; i++) {
        cJSON_AddItemToArray(headers, kv_to_json(&req->headers[i]));
    }
    cJSON_AddItemToObject(obj, "headers", headers);

    // Body
    cJSON* body = cJSON_CreateObject();
    cJSON_AddStringToObject(body, "body_type", body_type_to_string(req->body.body_type));
    cJSON_AddStringToObject(body, "content", req->body.content ? req->body.content : "");
    cJSON_AddStringToObject(body, "binary_file_path", req->body.binary_file_path);
    cJSON* urlencoded = cJSON_CreateArray();
    for (size_t i = 0; i < req->body.urlencoded_count; i++) {
        cJSON_AddItemToArray(urlencoded, kv_to_json(&req->body.urlencoded[i]));
    }
    cJSON_AddItemToObject(body, "urlencoded", urlencoded);
    cJSON_AddItemToObject(obj, "body", body);

    // Auth
    cJSON* auth = cJSON_CreateObject();
    cJSON_AddStringToObject(auth, "auth_type", auth_type_to_string(req->auth.type));
    cJSON_AddStringToObject(auth, "bearer_token", req->auth.bearer_token);
    cJSON_AddStringToObject(auth, "bearer_prefix", req->auth.bearer_prefix);
    cJSON_AddStringToObject(auth, "basic_username", req->auth.basic_username);
    cJSON_AddStringToObject(auth, "basic_password", req->auth.basic_password);
    cJSON_AddStringToObject(auth, "api_key_name", req->auth.api_key_name);
    cJSON_AddStringToObject(auth, "api_key_value", req->auth.api_key_value);
    cJSON_AddStringToObject(auth, "api_key_location", req->auth.api_key_location);
    cJSON_AddItemToObject(obj, "auth", auth);

    return obj;
}

ApiRequest* api_request_from_json(const cJSON* obj) {
    if (!obj) return NULL;
    ApiRequest* req = calloc(1, sizeof(ApiRequest));
    cJSON* item;

    if ((item = cJSON_GetObjectItem(obj, "id")) && item->valuestring) strncpy(req->id, item->valuestring, sizeof(req->id) - 1);
    else generate_uuid(req->id, sizeof(req->id));

    if ((item = cJSON_GetObjectItem(obj, "name")) && item->valuestring) strncpy(req->name, item->valuestring, sizeof(req->name) - 1);
    if ((item = cJSON_GetObjectItem(obj, "method")) && item->valuestring) req->method = http_method_from_string(item->valuestring);
    if ((item = cJSON_GetObjectItem(obj, "url")) && item->valuestring) strncpy(req->url, item->valuestring, sizeof(req->url) - 1);
    if ((item = cJSON_GetObjectItem(obj, "description")) && item->valuestring) strncpy(req->description, item->valuestring, sizeof(req->description) - 1);
    if ((item = cJSON_GetObjectItem(obj, "sort_order"))) req->sort_order = item->valueint;

    // Params
    cJSON* params = cJSON_GetObjectItem(obj, "params");
    if (params && cJSON_IsArray(params)) {
        int sz = cJSON_GetArraySize(params);
        req->params = malloc(sizeof(KeyValue) * sz);
        req->params_count = sz;
        for (int i = 0; i < sz; i++) {
            req->params[i] = kv_from_json(cJSON_GetArrayItem(params, i));
        }
    }

    // Headers
    cJSON* headers = cJSON_GetObjectItem(obj, "headers");
    if (headers && cJSON_IsArray(headers)) {
        int sz = cJSON_GetArraySize(headers);
        req->headers = malloc(sizeof(KeyValue) * sz);
        req->headers_count = sz;
        for (int i = 0; i < sz; i++) {
            req->headers[i] = kv_from_json(cJSON_GetArrayItem(headers, i));
        }
    }

    // Body
    cJSON* body = cJSON_GetObjectItem(obj, "body");
    if (body) {
        if ((item = cJSON_GetObjectItem(body, "body_type")) && item->valuestring) req->body.body_type = body_type_from_string(item->valuestring);
        if ((item = cJSON_GetObjectItem(body, "content")) && item->valuestring) req->body.content = strdup(item->valuestring);
        else req->body.content = strdup("");
        if ((item = cJSON_GetObjectItem(body, "binary_file_path")) && item->valuestring) strncpy(req->body.binary_file_path, item->valuestring, sizeof(req->body.binary_file_path) - 1);
        
        cJSON* urlencoded = cJSON_GetObjectItem(body, "urlencoded");
        if (urlencoded && cJSON_IsArray(urlencoded)) {
            int sz = cJSON_GetArraySize(urlencoded);
            req->body.urlencoded = malloc(sizeof(KeyValue) * sz);
            req->body.urlencoded_count = sz;
            for (int i = 0; i < sz; i++) {
                req->body.urlencoded[i] = kv_from_json(cJSON_GetArrayItem(urlencoded, i));
            }
        }
    } else {
        req->body.body_type = BODY_NO_BODY;
        req->body.content = strdup("");
    }

    // Auth
    cJSON* auth = cJSON_GetObjectItem(obj, "auth");
    if (auth) {
        if ((item = cJSON_GetObjectItem(auth, "auth_type")) && item->valuestring) req->auth.type = auth_type_from_string(item->valuestring);
        if ((item = cJSON_GetObjectItem(auth, "bearer_token")) && item->valuestring) strncpy(req->auth.bearer_token, item->valuestring, sizeof(req->auth.bearer_token) - 1);
        if ((item = cJSON_GetObjectItem(auth, "bearer_prefix")) && item->valuestring) strncpy(req->auth.bearer_prefix, item->valuestring, sizeof(req->auth.bearer_prefix) - 1);
        if ((item = cJSON_GetObjectItem(auth, "basic_username")) && item->valuestring) strncpy(req->auth.basic_username, item->valuestring, sizeof(req->auth.basic_username) - 1);
        if ((item = cJSON_GetObjectItem(auth, "basic_password")) && item->valuestring) strncpy(req->auth.basic_password, item->valuestring, sizeof(req->auth.basic_password) - 1);
        if ((item = cJSON_GetObjectItem(auth, "api_key_name")) && item->valuestring) strncpy(req->auth.api_key_name, item->valuestring, sizeof(req->auth.api_key_name) - 1);
        if ((item = cJSON_GetObjectItem(auth, "api_key_value")) && item->valuestring) strncpy(req->auth.api_key_value, item->valuestring, sizeof(req->auth.api_key_value) - 1);
        if ((item = cJSON_GetObjectItem(auth, "api_key_location")) && item->valuestring) strncpy(req->auth.api_key_location, item->valuestring, sizeof(req->auth.api_key_location) - 1);
    }

    return req;
}

static cJSON* folder_to_json(const Folder* f);
static Folder* folder_from_json(const cJSON* obj);

static cJSON* collection_item_to_json(const CollectionItem* item) {
    if (!item) return NULL;
    cJSON* obj = cJSON_CreateObject();
    if (item->type == ITEM_REQUEST) {
        cJSON_AddStringToObject(obj, "type", "request");
        cJSON_AddItemToObject(obj, "request", api_request_to_json(item->request));
    } else if (item->type == ITEM_FOLDER) {
        cJSON_AddStringToObject(obj, "type", "folder");
        cJSON_AddItemToObject(obj, "folder", folder_to_json(item->folder));
    }
    return obj;
}

static CollectionItem* collection_item_from_json(const cJSON* obj) {
    if (!obj) return NULL;
    cJSON* type_item = cJSON_GetObjectItem(obj, "type");
    if (!type_item || !type_item->valuestring) return NULL;

    CollectionItem* item = calloc(1, sizeof(CollectionItem));
    if (strcmp(type_item->valuestring, "request") == 0) {
        item->type = ITEM_REQUEST;
        item->request = api_request_from_json(cJSON_GetObjectItem(obj, "request"));
    } else if (strcmp(type_item->valuestring, "folder") == 0) {
        item->type = ITEM_FOLDER;
        item->folder = folder_from_json(cJSON_GetObjectItem(obj, "folder"));
    }
    return item;
}

static cJSON* folder_to_json(const Folder* f) {
    if (!f) return NULL;
    cJSON* obj = cJSON_CreateObject();
    cJSON_AddStringToObject(obj, "id", f->id);
    cJSON_AddStringToObject(obj, "name", f->name);
    cJSON_AddStringToObject(obj, "description", f->description);
    cJSON_AddBoolToObject(obj, "is_expanded", f->is_expanded);

    cJSON* items = cJSON_CreateArray();
    for (size_t i = 0; i < f->items_count; i++) {
        cJSON_AddItemToArray(items, collection_item_to_json(f->items[i]));
    }
    cJSON_AddItemToObject(obj, "items", items);
    return obj;
}

static Folder* folder_from_json(const cJSON* obj) {
    if (!obj) return NULL;
    Folder* f = calloc(1, sizeof(Folder));
    cJSON* item;
    if ((item = cJSON_GetObjectItem(obj, "id")) && item->valuestring) strncpy(f->id, item->valuestring, sizeof(f->id) - 1);
    else generate_uuid(f->id, sizeof(f->id));
    if ((item = cJSON_GetObjectItem(obj, "name")) && item->valuestring) strncpy(f->name, item->valuestring, sizeof(f->name) - 1);
    if ((item = cJSON_GetObjectItem(obj, "description")) && item->valuestring) strncpy(f->description, item->valuestring, sizeof(f->description) - 1);
    if ((item = cJSON_GetObjectItem(obj, "is_expanded"))) f->is_expanded = cJSON_IsTrue(item);
    else f->is_expanded = true;

    cJSON* items = cJSON_GetObjectItem(obj, "items");
    if (items && cJSON_IsArray(items)) {
        int sz = cJSON_GetArraySize(items);
        f->items = malloc(sizeof(CollectionItem*) * sz);
        f->items_count = 0;
        for (int i = 0; i < sz; i++) {
            CollectionItem* ci = collection_item_from_json(cJSON_GetArrayItem(items, i));
            if (ci) f->items[f->items_count++] = ci;
        }
    }
    return f;
}

static cJSON* collection_to_json(const Collection* col) {
    if (!col) return NULL;
    cJSON* obj = cJSON_CreateObject();
    cJSON_AddStringToObject(obj, "id", col->id);
    cJSON_AddStringToObject(obj, "name", col->name);
    cJSON_AddStringToObject(obj, "description", col->description);
    cJSON_AddBoolToObject(obj, "is_expanded", col->is_expanded);

    cJSON* items = cJSON_CreateArray();
    for (size_t i = 0; i < col->items_count; i++) {
        cJSON_AddItemToArray(items, collection_item_to_json(col->items[i]));
    }
    cJSON_AddItemToObject(obj, "items", items);
    return obj;
}

static Collection* collection_from_json(const cJSON* obj) {
    if (!obj) return NULL;
    Collection* col = calloc(1, sizeof(Collection));
    cJSON* item;
    if ((item = cJSON_GetObjectItem(obj, "id")) && item->valuestring) strncpy(col->id, item->valuestring, sizeof(col->id) - 1);
    else generate_uuid(col->id, sizeof(col->id));
    if ((item = cJSON_GetObjectItem(obj, "name")) && item->valuestring) strncpy(col->name, item->valuestring, sizeof(col->name) - 1);
    if ((item = cJSON_GetObjectItem(obj, "description")) && item->valuestring) strncpy(col->description, item->valuestring, sizeof(col->description) - 1);
    if ((item = cJSON_GetObjectItem(obj, "is_expanded"))) col->is_expanded = cJSON_IsTrue(item);
    else col->is_expanded = true;

    cJSON* items = cJSON_GetObjectItem(obj, "items");
    if (items && cJSON_IsArray(items)) {
        int sz = cJSON_GetArraySize(items);
        col->items = malloc(sizeof(CollectionItem*) * sz);
        col->items_count = 0;
        for (int i = 0; i < sz; i++) {
            CollectionItem* ci = collection_item_from_json(cJSON_GetArrayItem(items, i));
            if (ci) col->items[col->items_count++] = ci;
        }
    }
    return col;
}

cJSON* workspace_to_json(const Workspace* ws) {
    if (!ws) return NULL;
    cJSON* obj = cJSON_CreateObject();
    cJSON_AddStringToObject(obj, "id", ws->id);
    cJSON_AddStringToObject(obj, "name", ws->name);

    cJSON* collections = cJSON_CreateArray();
    for (size_t i = 0; i < ws->collections_count; i++) {
        cJSON_AddItemToArray(collections, collection_to_json(ws->collections[i]));
    }
    cJSON_AddItemToObject(obj, "collections", collections);

    cJSON* variables = cJSON_CreateArray();
    for (size_t i = 0; i < ws->variables_count; i++) {
        cJSON_AddItemToArray(variables, kv_to_json(&ws->variables[i]));
    }
    cJSON_AddItemToObject(obj, "variables", variables);

    return obj;
}

Workspace* workspace_from_json(const cJSON* obj) {
    if (!obj) return NULL;
    Workspace* ws = calloc(1, sizeof(Workspace));
    cJSON* item;
    if ((item = cJSON_GetObjectItem(obj, "id")) && item->valuestring) strncpy(ws->id, item->valuestring, sizeof(ws->id) - 1);
    else generate_uuid(ws->id, sizeof(ws->id));
    if ((item = cJSON_GetObjectItem(obj, "name")) && item->valuestring) strncpy(ws->name, item->valuestring, sizeof(ws->name) - 1);

    cJSON* collections = cJSON_GetObjectItem(obj, "collections");
    if (collections && cJSON_IsArray(collections)) {
        int sz = cJSON_GetArraySize(collections);
        ws->collections = malloc(sizeof(Collection*) * sz);
        ws->collections_count = 0;
        for (int i = 0; i < sz; i++) {
            Collection* c = collection_from_json(cJSON_GetArrayItem(collections, i));
            if (c) ws->collections[ws->collections_count++] = c;
        }
    }

    cJSON* variables = cJSON_GetObjectItem(obj, "variables");
    if (variables && cJSON_IsArray(variables)) {
        int sz = cJSON_GetArraySize(variables);
        ws->variables = malloc(sizeof(KeyValue) * sz);
        ws->variables_count = sz;
        for (int i = 0; i < sz; i++) {
            ws->variables[i] = kv_from_json(cJSON_GetArrayItem(variables, i));
        }
    }

    return ws;
}

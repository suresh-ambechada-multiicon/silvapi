#include "app/import_export.h"
#include <ctype.h>
#include <strings.h>

Collection* import_openapi_json(const char* json_content) {
    if (!json_content) return NULL;
    cJSON* root = cJSON_Parse(json_content);
    if (!root) return NULL;

    cJSON* info = cJSON_GetObjectItem(root, "info");
    const char* title = "OpenAPI Collection";
    if (info) {
        cJSON* t = cJSON_GetObjectItem(info, "title");
        if (t && t->valuestring) title = t->valuestring;
    }

    Collection* col = collection_create(title);

    // Get Base URL from servers[0].url if present
    cJSON* servers = cJSON_GetObjectItem(root, "servers");
    const char* server_url = "https://api.example.com";
    if (servers && cJSON_IsArray(servers) && cJSON_GetArraySize(servers) > 0) {
        cJSON* s0 = cJSON_GetArrayItem(servers, 0);
        cJSON* u = cJSON_GetObjectItem(s0, "url");
        if (u && u->valuestring) server_url = u->valuestring;
    }
    kv_array_add(&col->variables, &col->variables_count, "baseUrl", server_url, true);

    cJSON* paths = cJSON_GetObjectItem(root, "paths");
    if (paths && cJSON_IsObject(paths)) {
        cJSON* path_item = paths->child;
        while (path_item) {
            const char* path_str = path_item->string;

            cJSON* method_item = path_item->child;
            while (method_item) {
                const char* method_name = method_item->string;
                if (strcasecmp(method_name, "get") == 0 ||
                    strcasecmp(method_name, "post") == 0 ||
                    strcasecmp(method_name, "put") == 0 ||
                    strcasecmp(method_name, "patch") == 0 ||
                    strcasecmp(method_name, "delete") == 0 ||
                    strcasecmp(method_name, "head") == 0 ||
                    strcasecmp(method_name, "options") == 0) {
                    
                    cJSON* summary = cJSON_GetObjectItem(method_item, "summary");
                    cJSON* op_id = cJSON_GetObjectItem(method_item, "operationId");
                    char req_name[256];
                    if (summary && summary->valuestring) {
                        strncpy(req_name, summary->valuestring, sizeof(req_name) - 1);
                    } else if (op_id && op_id->valuestring) {
                        strncpy(req_name, op_id->valuestring, sizeof(req_name) - 1);
                    } else {
                        snprintf(req_name, sizeof(req_name), "%s %s", method_name, path_str);
                    }

                    char full_url[1024];
                    snprintf(full_url, sizeof(full_url), "{{baseUrl}}%s", path_str);

                    ApiRequest* req = api_request_create(req_name, http_method_from_string(method_name), full_url);

                    // Add parameters (query/path/header)
                    cJSON* parameters = cJSON_GetObjectItem(method_item, "parameters");
                    if (parameters && cJSON_IsArray(parameters)) {
                        int p_sz = cJSON_GetArraySize(parameters);
                        for (int pi = 0; pi < p_sz; pi++) {
                            cJSON* p = cJSON_GetArrayItem(parameters, pi);
                            cJSON* pname = cJSON_GetObjectItem(p, "name");
                            cJSON* pin = cJSON_GetObjectItem(p, "in");
                            if (pname && pname->valuestring && pin && pin->valuestring) {
                                if (strcmp(pin->valuestring, "query") == 0 || strcmp(pin->valuestring, "path") == 0) {
                                    kv_array_add(&req->params, &req->params_count, pname->valuestring, "", true);
                                } else if (strcmp(pin->valuestring, "header") == 0) {
                                    kv_array_add(&req->headers, &req->headers_count, pname->valuestring, "", true);
                                }
                            }
                        }
                    }

                    // Add request body schema preview if JSON
                    cJSON* req_body = cJSON_GetObjectItem(method_item, "requestBody");
                    if (req_body) {
                        req->body.body_type = BODY_JSON;
                        free(req->body.content);
                        req->body.content = strdup("{\n  \n}");
                        kv_array_add(&req->headers, &req->headers_count, "Content-Type", "application/json", true);
                    }

                    // Add item to collection
                    CollectionItem* item = calloc(1, sizeof(CollectionItem));
                    item->type = ITEM_REQUEST;
                    item->request = req;

                    col->items = realloc(col->items, sizeof(CollectionItem*) * (col->items_count + 1));
                    col->items[col->items_count++] = item;
                }
                method_item = method_item->next;
            }
            path_item = path_item->next;
        }
    }

    cJSON_Delete(root);
    return col;
}

static void parse_postman_items(cJSON* items_array, CollectionItem*** out_items, size_t* out_count) {
    if (!items_array || !cJSON_IsArray(items_array)) return;
    int sz = cJSON_GetArraySize(items_array);

    for (int i = 0; i < sz; i++) {
        cJSON* it = cJSON_GetArrayItem(items_array, i);
        cJSON* it_name = cJSON_GetObjectItem(it, "name");
        cJSON* sub_items = cJSON_GetObjectItem(it, "item");
        cJSON* request_obj = cJSON_GetObjectItem(it, "request");

        if (sub_items && cJSON_IsArray(sub_items)) {
            // It's a folder
            Folder* folder = folder_create(it_name && it_name->valuestring ? it_name->valuestring : "Folder");
            parse_postman_items(sub_items, &folder->items, &folder->items_count);

            CollectionItem* ci = calloc(1, sizeof(CollectionItem));
            ci->type = ITEM_FOLDER;
            ci->folder = folder;

            *out_items = realloc(*out_items, sizeof(CollectionItem*) * (*out_count + 1));
            (*out_items)[(*out_count)++] = ci;
        } else if (request_obj) {
            // It's a request
            cJSON* method = cJSON_GetObjectItem(request_obj, "method");
            cJSON* url = cJSON_GetObjectItem(request_obj, "url");
            const char* url_str = "";
            if (url) {
                if (cJSON_IsString(url)) url_str = url->valuestring;
                else if (cJSON_IsObject(url)) {
                    cJSON* raw = cJSON_GetObjectItem(url, "raw");
                    if (raw && raw->valuestring) url_str = raw->valuestring;
                }
            }

            ApiRequest* req = api_request_create(it_name && it_name->valuestring ? it_name->valuestring : "Request",
                                                 method && method->valuestring ? http_method_from_string(method->valuestring) : METHOD_GET,
                                                 url_str);

            // Headers
            cJSON* headers = cJSON_GetObjectItem(request_obj, "header");
            if (headers && cJSON_IsArray(headers)) {
                int h_sz = cJSON_GetArraySize(headers);
                for (int hi = 0; hi < h_sz; hi++) {
                    cJSON* h = cJSON_GetArrayItem(headers, hi);
                    cJSON* k = cJSON_GetObjectItem(h, "key");
                    cJSON* v = cJSON_GetObjectItem(h, "value");
                    cJSON* dis = cJSON_GetObjectItem(h, "disabled");
                    bool en = dis ? !cJSON_IsTrue(dis) : true;
                    if (k && k->valuestring) {
                        kv_array_add(&req->headers, &req->headers_count, k->valuestring, v && v->valuestring ? v->valuestring : "", en);
                    }
                }
            }

            // Body
            cJSON* body = cJSON_GetObjectItem(request_obj, "body");
            if (body && cJSON_IsObject(body)) {
                cJSON* mode = cJSON_GetObjectItem(body, "mode");
                if (mode && mode->valuestring) {
                    if (strcmp(mode->valuestring, "raw") == 0) {
                        cJSON* raw = cJSON_GetObjectItem(body, "raw");
                        req->body.body_type = BODY_JSON;
                        if (raw && raw->valuestring) {
                            free(req->body.content);
                            req->body.content = strdup(raw->valuestring);
                        }
                    } else if (strcmp(mode->valuestring, "urlencoded") == 0) {
                        req->body.body_type = BODY_URLENCODED;
                        cJSON* uarr = cJSON_GetObjectItem(body, "urlencoded");
                        if (uarr && cJSON_IsArray(uarr)) {
                            int u_sz = cJSON_GetArraySize(uarr);
                            for (int ui = 0; ui < u_sz; ui++) {
                                cJSON* uitem = cJSON_GetArrayItem(uarr, ui);
                                cJSON* uk = cJSON_GetObjectItem(uitem, "key");
                                cJSON* uv = cJSON_GetObjectItem(uitem, "value");
                                cJSON* udis = cJSON_GetObjectItem(uitem, "disabled");
                                bool en = udis ? !cJSON_IsTrue(udis) : true;
                                if (uk && uk->valuestring) {
                                    kv_array_add(&req->body.urlencoded, &req->body.urlencoded_count, uk->valuestring, uv && uv->valuestring ? uv->valuestring : "", en);
                                }
                            }
                        }
                    }
                }
            }

            CollectionItem* ci = calloc(1, sizeof(CollectionItem));
            ci->type = ITEM_REQUEST;
            ci->request = req;

            *out_items = realloc(*out_items, sizeof(CollectionItem*) * (*out_count + 1));
            (*out_items)[(*out_count)++] = ci;
        }
    }
}

Collection* import_postman_json(const char* json_content) {
    if (!json_content) return NULL;
    cJSON* root = cJSON_Parse(json_content);
    if (!root) return NULL;

    cJSON* info = cJSON_GetObjectItem(root, "info");
    const char* col_name = "Postman Collection";
    if (info) {
        cJSON* n = cJSON_GetObjectItem(info, "name");
        if (n && n->valuestring) col_name = n->valuestring;
    }

    Collection* col = collection_create(col_name);
    cJSON* items = cJSON_GetObjectItem(root, "item");
    if (items) {
        parse_postman_items(items, &col->items, &col->items_count);
    }

    cJSON_Delete(root);
    return col;
}

ApiRequest* import_curl_command(const char* cmd) {
    if (!cmd || !*cmd) return NULL;
    ApiRequest* req = api_request_create("Imported cURL", METHOD_GET, "");

    const char* p = cmd;
    while (*p) {
        while (*p && isspace((unsigned char)*p)) p++;
        if (!*p) break;

        if (strncmp(p, "-X ", 3) == 0 || strncmp(p, "--request ", 10) == 0) {
            p += (p[1] == 'X') ? 3 : 10;
            while (*p && isspace((unsigned char)*p)) p++;
            char m[32] = {0};
            int mi = 0;
            while (*p && !isspace((unsigned char)*p) && mi < 31) m[mi++] = *p++;
            req->method = http_method_from_string(m);
        } else if (strncmp(p, "-H ", 3) == 0 || strncmp(p, "--header ", 9) == 0) {
            p += (p[1] == 'H') ? 3 : 9;
            while (*p && isspace((unsigned char)*p)) p++;
            char quote = (*p == '\'' || *p == '"') ? *p++ : 0;
            char header_str[2048] = {0};
            int hi = 0;
            while (*p && ((quote && *p != quote) || (!quote && !isspace((unsigned char)*p))) && hi < 2047) {
                header_str[hi++] = *p++;
            }
            if (quote && *p == quote) p++;

            char* colon = strchr(header_str, ':');
            if (colon) {
                *colon = '\0';
                char* val = colon + 1;
                while (*val && isspace((unsigned char)*val)) val++;
                kv_array_add(&req->headers, &req->headers_count, header_str, val, true);
            }
        } else if (strncmp(p, "-d ", 3) == 0 || strncmp(p, "--data ", 7) == 0 || strncmp(p, "--data-raw ", 11) == 0) {
            if (req->method == METHOD_GET) req->method = METHOD_POST;
            p = strchr(p, ' ') + 1;
            while (*p && isspace((unsigned char)*p)) p++;
            char quote = (*p == '\'' || *p == '"') ? *p++ : 0;
            char data_str[4096] = {0};
            int di = 0;
            while (*p && ((quote && *p != quote) || (!quote && !isspace((unsigned char)*p))) && di < 4095) {
                data_str[di++] = *p++;
            }
            if (quote && *p == quote) p++;

            req->body.body_type = (data_str[0] == '{' || data_str[0] == '[') ? BODY_JSON : BODY_RAW;
            free(req->body.content);
            req->body.content = strdup(data_str);
        } else if (strncmp(p, "http://", 7) == 0 || strncmp(p, "https://", 8) == 0 || (*p == '"' || *p == '\'')) {
            char quote = (*p == '\'' || *p == '"') ? *p++ : 0;
            char url_str[2048] = {0};
            int ui = 0;
            while (*p && ((quote && *p != quote) || (!quote && !isspace((unsigned char)*p))) && ui < 2047) {
                url_str[ui++] = *p++;
            }
            if (quote && *p == quote) p++;
            if (url_str[0]) {
                strncpy(req->url, url_str, sizeof(req->url) - 1);
            }
        } else {
            // Skip word
            while (*p && !isspace((unsigned char)*p)) p++;
        }
    }

    return req;
}

char* export_curl_command(const ApiRequest* req, const Workspace* ws, const Collection* col, const Folder* folder) {
    if (!req) return strdup("");

    size_t cap = 2048;
    char* cmd = malloc(cap);
    size_t len = 0;
    cmd[0] = '\0';

    char* url = interpolate_variables(req->url, ws, col, folder);
    snprintf(cmd, cap, "curl -X %s \"%s\"", http_method_to_string(req->method), url);
    len = strlen(cmd);
    free(url);

    // Headers
    for (size_t i = 0; i < req->headers_count; i++) {
        if (!req->headers[i].enabled || req->headers[i].key[0] == '\0') continue;
        char* k = interpolate_variables(req->headers[i].key, ws, col, folder);
        char* v = interpolate_variables(req->headers[i].value, ws, col, folder);
        char hbuf[2048];
        snprintf(hbuf, sizeof(hbuf), " \\\n  -H \"%s: %s\"", k, v);
        size_t hlen = strlen(hbuf);
        while (len + hlen + 16 >= cap) {
            cap *= 2;
            cmd = realloc(cmd, cap);
        }
        strcpy(cmd + len, hbuf);
        len += hlen;
        free(k);
        free(v);
    }

    // Body
    if (req->method != METHOD_GET && req->method != METHOD_HEAD && req->body.content && *req->body.content) {
        char* b = interpolate_variables(req->body.content, ws, col, folder);
        char bbuf[4096];
        snprintf(bbuf, sizeof(bbuf), " \\\n  -d '%s'", b);
        size_t blen = strlen(bbuf);
        while (len + blen + 16 >= cap) {
            cap *= 2;
            cmd = realloc(cmd, cap);
        }
        strcpy(cmd + len, bbuf);
        len += blen;
        free(b);
    }

    return cmd;
}

Collection* import_auto_detect(const char* content, char* out_type_detected, size_t out_type_sz) {
    if (!content) return NULL;

    // Skip whitespace
    while (*content && isspace((unsigned char)*content)) content++;
    if (!*content) return NULL;

    // 1. Check for cURL
    if (strncmp(content, "curl", 4) == 0) {
        ApiRequest* req = import_curl_command(content);
        if (req) {
            if (out_type_detected) snprintf(out_type_detected, out_type_sz, "cURL Command");
            Collection* col = collection_create("Imported cURL");
            CollectionItem* ci = calloc(1, sizeof(CollectionItem));
            ci->type = ITEM_REQUEST;
            ci->request = req;
            col->items = calloc(1, sizeof(CollectionItem*));
            col->items[0] = ci;
            col->items_count = 1;
            return col;
        }
    }

    // 2. Check for URL
    if (strncmp(content, "http://", 7) == 0 || strncmp(content, "https://", 8) == 0) {
        if (out_type_detected) snprintf(out_type_detected, out_type_sz, "Raw URL");
        Collection* col = collection_create("Imported URL");
        ApiRequest* req = api_request_create("Request", METHOD_GET, content);
        CollectionItem* ci = calloc(1, sizeof(CollectionItem));
        ci->type = ITEM_REQUEST;
        ci->request = req;
        col->items = calloc(1, sizeof(CollectionItem*));
        col->items[0] = ci;
        col->items_count = 1;
        return col;
    }

    // 3. Check JSON (OpenAPI vs Postman)
    if (*content == '{' || *content == '[') {
        if (strstr(content, "\"openapi\"") || strstr(content, "\"swagger\"") || strstr(content, "'openapi'")) {
            Collection* col = import_openapi_json(content);
            if (col) {
                if (out_type_detected) snprintf(out_type_detected, out_type_sz, "OpenAPI / Swagger");
                return col;
            }
        }

        if (strstr(content, "postman") || strstr(content, "\"item\"") || strstr(content, "_postman_id")) {
            Collection* col = import_postman_json(content);
            if (col) {
                if (out_type_detected) snprintf(out_type_detected, out_type_sz, "Postman Collection");
                return col;
            }
        }

        // Try both
        Collection* col = import_openapi_json(content);
        if (col) {
            if (out_type_detected) snprintf(out_type_detected, out_type_sz, "OpenAPI Specification");
            return col;
        }
        col = import_postman_json(content);
        if (col) {
            if (out_type_detected) snprintf(out_type_detected, out_type_sz, "Postman Collection");
            return col;
        }
    }

    return NULL;
}


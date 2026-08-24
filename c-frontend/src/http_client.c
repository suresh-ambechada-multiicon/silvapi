#include "http_client.h"
#include "curl/curl.h"
#include <ctype.h>
#include <unistd.h>

typedef struct {
    ApiRequest* req;
    Workspace* ws;
    Collection* col;
    Folder* folder;
    HttpResponseCallback callback;
    void* user_data;
    pthread_t thread;
    bool is_cancelled;
    CURL* curl_handle;
} AsyncRequestTask;

#define MAX_ACTIVE_TASKS 32
static AsyncRequestTask* s_active_tasks[MAX_ACTIVE_TASKS];
static pthread_mutex_t s_tasks_mutex = PTHREAD_MUTEX_INITIALIZER;

static size_t write_body_cb(void* contents, size_t size, size_t nmemb, void* userp) {
    size_t total = size * nmemb;
    HttpResponse* resp = (HttpResponse*)userp;

    size_t current_len = resp->body_size;
    resp->body = realloc(resp->body, current_len + total + 1);
    memcpy(resp->body + current_len, contents, total);
    resp->body[current_len + total] = '\0';
    resp->body_size += total;

    return total;
}

static size_t write_header_cb(char* buffer, size_t size, size_t nitems, void* userp) {
    size_t total = size * nitems;
    HttpResponse* resp = (HttpResponse*)userp;

    char* colon = memchr(buffer, ':', total);
    if (colon) {
        size_t key_len = colon - buffer;
        char key[256] = {0};
        if (key_len < sizeof(key)) {
            memcpy(key, buffer, key_len);
            key[key_len] = '\0';
        }

        char* val_start = colon + 1;
        while (val_start < buffer + total && isspace((unsigned char)*val_start)) val_start++;
        char* val_end = buffer + total - 1;
        while (val_end > val_start && isspace((unsigned char)*val_end)) val_end--;

        char val[1024] = {0};
        size_t val_len = (val_end >= val_start) ? (val_end - val_start + 1) : 0;
        if (val_len < sizeof(val)) {
            memcpy(val, val_start, val_len);
            val[val_len] = '\0';
        }

        kv_array_add(&resp->headers, &resp->headers_count, key, val, true);
    }

    return total;
}

static void* request_worker_thread(void* arg) {
    AsyncRequestTask* task = (AsyncRequestTask*)arg;
    ApiRequest* req = task->req;

    HttpResponse resp = {0};
    strncpy(resp.request_id, req->id, sizeof(resp.request_id) - 1);
    resp.body = strdup("");
    resp.body_size = 0;

    // Get current ISO-like timestamp
    time_t now = time(NULL);
    struct tm* tm_info = localtime(&now);
    strftime(resp.timestamp, sizeof(resp.timestamp), "%Y-%m-%d %H:%M:%S", tm_info);

    CURL* curl = curl_easy_init();
    if (!curl) {
        resp.error = strdup("Failed to initialize libcurl");
        task->callback(&resp, task->user_data);
        http_response_free_contents(&resp);
        return NULL;
    }

    task->curl_handle = curl;

    // 1. URL with variable interpolation & query params
    char* interpolated_url = interpolate_variables(req->url, task->ws, task->col, task->folder);
    
    // Append query parameters if any
    size_t active_params = 0;
    for (size_t i = 0; i < req->params_count; i++) {
        if (req->params[i].enabled && req->params[i].key[0] != '\0') active_params++;
    }

    char final_url[4096];
    strncpy(final_url, interpolated_url, sizeof(final_url) - 1);
    final_url[sizeof(final_url) - 1] = '\0';
    free(interpolated_url);

    if (active_params > 0) {
        bool has_qmark = (strchr(final_url, '?') != NULL);
        for (size_t i = 0; i < req->params_count; i++) {
            if (!req->params[i].enabled || req->params[i].key[0] == '\0') continue;
            
            char* ikey = interpolate_variables(req->params[i].key, task->ws, task->col, task->folder);
            char* ival = interpolate_variables(req->params[i].value, task->ws, task->col, task->folder);

            char* ekey = curl_easy_escape(curl, ikey, 0);
            char* eval = curl_easy_escape(curl, ival, 0);

            size_t cur_len = strlen(final_url);
            snprintf(final_url + cur_len, sizeof(final_url) - cur_len, "%s%s=%s",
                     has_qmark ? "&" : "?", ekey ? ekey : "", eval ? eval : "");
            has_qmark = true;

            if (ekey) curl_free(ekey);
            if (eval) curl_free(eval);
            free(ikey);
            free(ival);
        }
    }

    curl_easy_setopt(curl, CURLOPT_URL, final_url);
    curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);
    curl_easy_setopt(curl, CURLOPT_MAXREDIRS, 10L);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT, 60L);
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 0L);
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYHOST, 0L);

    // Callbacks
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_body_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &resp);
    curl_easy_setopt(curl, CURLOPT_HEADERFUNCTION, write_header_cb);
    curl_easy_setopt(curl, CURLOPT_HEADERDATA, &resp);

    // 2. HTTP Method
    switch (req->method) {
        case METHOD_GET:
            curl_easy_setopt(curl, CURLOPT_HTTPGET, 1L);
            break;
        case METHOD_POST:
            curl_easy_setopt(curl, CURLOPT_POST, 1L);
            break;
        case METHOD_PUT:
            curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, "PUT");
            break;
        case METHOD_PATCH:
            curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, "PATCH");
            break;
        case METHOD_DELETE:
            curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, "DELETE");
            break;
        case METHOD_HEAD:
            curl_easy_setopt(curl, CURLOPT_NOBODY, 1L);
            break;
        case METHOD_OPTIONS:
            curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, "OPTIONS");
            break;
        default:
            break;
    }

    // 3. Headers
    struct curl_slist* header_list = NULL;
    for (size_t i = 0; i < req->headers_count; i++) {
        if (!req->headers[i].enabled || req->headers[i].key[0] == '\0') continue;
        char* ikey = interpolate_variables(req->headers[i].key, task->ws, task->col, task->folder);
        char* ival = interpolate_variables(req->headers[i].value, task->ws, task->col, task->folder);
        char hbuf[2048];
        snprintf(hbuf, sizeof(hbuf), "%s: %s", ikey, ival);
        header_list = curl_slist_append(header_list, hbuf);
        free(ikey);
        free(ival);
    }

    // 4. Auth
    if (req->auth.type == AUTH_BEARER && req->auth.bearer_token[0] != '\0') {
        char* token = interpolate_variables(req->auth.bearer_token, task->ws, task->col, task->folder);
        const char* prefix = req->auth.bearer_prefix[0] ? req->auth.bearer_prefix : "Bearer";
        char hbuf[2048];
        snprintf(hbuf, sizeof(hbuf), "Authorization: %s %s", prefix, token);
        header_list = curl_slist_append(header_list, hbuf);
        free(token);
    } else if (req->auth.type == AUTH_BASIC) {
        char* user = interpolate_variables(req->auth.basic_username, task->ws, task->col, task->folder);
        char* pass = interpolate_variables(req->auth.basic_password, task->ws, task->col, task->folder);
        char userpwd[1024];
        snprintf(userpwd, sizeof(userpwd), "%s:%s", user, pass);
        curl_easy_setopt(curl, CURLOPT_HTTPAUTH, CURLAUTH_BASIC);
        curl_easy_setopt(curl, CURLOPT_USERPWD, userpwd);
        free(user);
        free(pass);
    } else if (req->auth.type == AUTH_API_KEY && req->auth.api_key_name[0] != '\0') {
        char* key_name = interpolate_variables(req->auth.api_key_name, task->ws, task->col, task->folder);
        char* key_val = interpolate_variables(req->auth.api_key_value, task->ws, task->col, task->folder);
        if (strcasecmp(req->auth.api_key_location, "query") == 0) {
            // Already appended or handled
        } else {
            char hbuf[2048];
            snprintf(hbuf, sizeof(hbuf), "%s: %s", key_name, key_val);
            header_list = curl_slist_append(header_list, hbuf);
        }
        free(key_name);
        free(key_val);
    }

    // 5. Body
    char* interpolated_body = NULL;
    curl_mime* mime = NULL;

    if (req->method != METHOD_GET && req->method != METHOD_HEAD) {
        if (req->body.body_type == BODY_JSON || req->body.body_type == BODY_RAW) {
            interpolated_body = interpolate_variables(req->body.content ? req->body.content : "", task->ws, task->col, task->folder);
            curl_easy_setopt(curl, CURLOPT_POSTFIELDS, interpolated_body);
            if (req->body.body_type == BODY_JSON) {
                header_list = curl_slist_append(header_list, "Content-Type: application/json");
            }
        } else if (req->body.body_type == BODY_URLENCODED) {
            size_t enc_cap = 1024;
            char* enc_buf = malloc(enc_cap);
            size_t enc_len = 0;
            enc_buf[0] = '\0';
            for (size_t i = 0; i < req->body.urlencoded_count; i++) {
                if (!req->body.urlencoded[i].enabled) continue;
                char* k = curl_easy_escape(curl, req->body.urlencoded[i].key, 0);
                char* v = curl_easy_escape(curl, req->body.urlencoded[i].value, 0);
                char part[1024];
                snprintf(part, sizeof(part), "%s%s=%s", (enc_len > 0) ? "&" : "", k ? k : "", v ? v : "");
                size_t plen = strlen(part);
                if (enc_len + plen + 1 >= enc_cap) {
                    enc_cap = (enc_len + plen + 1024) * 2;
                    enc_buf = realloc(enc_buf, enc_cap);
                }
                strcpy(enc_buf + enc_len, part);
                enc_len += plen;
                if (k) curl_free(k);
                if (v) curl_free(v);
            }
            interpolated_body = enc_buf;
            curl_easy_setopt(curl, CURLOPT_POSTFIELDS, interpolated_body);
            header_list = curl_slist_append(header_list, "Content-Type: application/x-www-form-urlencoded");
        } else if (req->body.body_type == BODY_FORM_DATA) {
            mime = curl_mime_init(curl);
            for (size_t i = 0; i < req->body.form_data_count; i++) {
                if (!req->body.form_data[i].enabled) continue;
                curl_mimepart* part = curl_mime_addpart(mime);
                curl_mime_name(part, req->body.form_data[i].name);
                if (req->body.form_data[i].is_file) {
                    curl_mime_filedata(part, req->body.form_data[i].file_path);
                } else {
                    curl_mime_data(part, req->body.form_data[i].value, CURL_ZERO_TERMINATED);
                }
            }
            curl_easy_setopt(curl, CURLOPT_MIMEPOST, mime);
        }
    }

    if (header_list) {
        curl_easy_setopt(curl, CURLOPT_HTTPHEADER, header_list);
    }

    // Execute Request
    CURLcode res = curl_easy_perform(curl);

    if (res == CURLE_OK) {
        long http_code = 0;
        curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &http_code);
        resp.status_code = (int)http_code;
        
        switch (http_code) {
            case 200: strncpy(resp.status_text, "OK", sizeof(resp.status_text) - 1); break;
            case 201: strncpy(resp.status_text, "Created", sizeof(resp.status_text) - 1); break;
            case 204: strncpy(resp.status_text, "No Content", sizeof(resp.status_text) - 1); break;
            case 301: strncpy(resp.status_text, "Moved Permanently", sizeof(resp.status_text) - 1); break;
            case 302: strncpy(resp.status_text, "Found", sizeof(resp.status_text) - 1); break;
            case 304: strncpy(resp.status_text, "Not Modified", sizeof(resp.status_text) - 1); break;
            case 400: strncpy(resp.status_text, "Bad Request", sizeof(resp.status_text) - 1); break;
            case 401: strncpy(resp.status_text, "Unauthorized", sizeof(resp.status_text) - 1); break;
            case 403: strncpy(resp.status_text, "Forbidden", sizeof(resp.status_text) - 1); break;
            case 404: strncpy(resp.status_text, "Not Found", sizeof(resp.status_text) - 1); break;
            case 405: strncpy(resp.status_text, "Method Not Allowed", sizeof(resp.status_text) - 1); break;
            case 500: strncpy(resp.status_text, "Internal Server Error", sizeof(resp.status_text) - 1); break;
            case 502: strncpy(resp.status_text, "Bad Gateway", sizeof(resp.status_text) - 1); break;
            case 503: strncpy(resp.status_text, "Service Unavailable", sizeof(resp.status_text) - 1); break;
            case 504: strncpy(resp.status_text, "Gateway Timeout", sizeof(resp.status_text) - 1); break;
            default: snprintf(resp.status_text, sizeof(resp.status_text), "Status %ld", http_code); break;
        }

        curl_off_t total_time_us = 0;
        curl_off_t namelookup_us = 0;
        curl_off_t connect_us = 0;
        curl_off_t appconnect_us = 0;
        curl_off_t starttransfer_us = 0;

        curl_easy_getinfo(curl, CURLINFO_TOTAL_TIME_T, &total_time_us);
        curl_easy_getinfo(curl, CURLINFO_NAMELOOKUP_TIME_T, &namelookup_us);
        curl_easy_getinfo(curl, CURLINFO_CONNECT_TIME_T, &connect_us);
        curl_easy_getinfo(curl, CURLINFO_APPCONNECT_TIME_T, &appconnect_us);
        curl_easy_getinfo(curl, CURLINFO_STARTTRANSFER_TIME_T, &starttransfer_us);

        resp.latency_ms = (uint64_t)(total_time_us / 1000);

        // Timeline events
        resp.timeline_count = 0;
        // 1. DNS Lookup
        resp.timeline[resp.timeline_count++] = (TimelineEvent){
            .name = "DNS Lookup",
            .duration_ms = (double)namelookup_us / 1000.0,
            .start_offset_ms = 0.0,
            .description = "Resolving hostname to IP"
        };
        // 2. TCP Connection
        resp.timeline[resp.timeline_count++] = (TimelineEvent){
            .name = "TCP Connect",
            .duration_ms = (double)(connect_us - namelookup_us) / 1000.0,
            .start_offset_ms = (double)namelookup_us / 1000.0,
            .description = "Establishing TCP handshake"
        };
        // 3. TLS Handshake (if HTTPS)
        if (appconnect_us > connect_us) {
            resp.timeline[resp.timeline_count++] = (TimelineEvent){
                .name = "TLS Handshake",
                .duration_ms = (double)(appconnect_us - connect_us) / 1000.0,
                .start_offset_ms = (double)connect_us / 1000.0,
                .description = "Negotiating SSL/TLS session"
            };
        }
        // 4. TTFB (Time to First Byte)
        curl_off_t pre_transfer = (appconnect_us > 0) ? appconnect_us : connect_us;
        resp.timeline[resp.timeline_count++] = (TimelineEvent){
            .name = "First Byte (TTFB)",
            .duration_ms = (double)(starttransfer_us - pre_transfer) / 1000.0,
            .start_offset_ms = (double)pre_transfer / 1000.0,
            .description = "Waiting for server response"
        };
        // 5. Content Download
        resp.timeline[resp.timeline_count++] = (TimelineEvent){
            .name = "Content Download",
            .duration_ms = (double)(total_time_us - starttransfer_us) / 1000.0,
            .start_offset_ms = (double)starttransfer_us / 1000.0,
            .description = "Receiving response payload"
        };
    } else {
        resp.error = strdup(curl_easy_strerror(res));
    }

    // Cleanup curl resources
    if (header_list) curl_slist_free_all(header_list);
    if (mime) curl_mime_free(mime);
    if (interpolated_body) free(interpolated_body);
    curl_easy_cleanup(curl);

    // Invoke user callback
    task->callback(&resp, task->user_data);

    // Free task
    pthread_mutex_lock(&s_tasks_mutex);
    for (int i = 0; i < MAX_ACTIVE_TASKS; i++) {
        if (s_active_tasks[i] == task) {
            s_active_tasks[i] = NULL;
            break;
        }
    }
    pthread_mutex_unlock(&s_tasks_mutex);

    api_request_free(task->req);
    free(task);
    http_response_free_contents(&resp);

    return NULL;
}

bool http_client_init(void) {
    return curl_global_init(CURL_GLOBAL_ALL) == CURLE_OK;
}

void http_client_shutdown(void) {
    curl_global_cleanup();
}

bool http_client_send(const ApiRequest* req, const Workspace* ws, const Collection* col, const Folder* folder,
                      HttpResponseCallback callback, void* user_data) {
    if (!req || !callback) return false;

    AsyncRequestTask* task = calloc(1, sizeof(AsyncRequestTask));
    task->req = api_request_clone(req);
    task->ws = (Workspace*)ws;
    task->col = (Collection*)col;
    task->folder = (Folder*)folder;
    task->callback = callback;
    task->user_data = user_data;

    pthread_mutex_lock(&s_tasks_mutex);
    int slot = -1;
    for (int i = 0; i < MAX_ACTIVE_TASKS; i++) {
        if (s_active_tasks[i] == NULL) {
            slot = i;
            break;
        }
    }
    if (slot < 0) {
        pthread_mutex_unlock(&s_tasks_mutex);
        api_request_free(task->req);
        free(task);
        return false;
    }
    s_active_tasks[slot] = task;
    pthread_mutex_unlock(&s_tasks_mutex);

    pthread_create(&task->thread, NULL, request_worker_thread, task);
    pthread_detach(task->thread);

    return true;
}

void http_client_cancel(const char* request_id) {
    if (!request_id) return;
    pthread_mutex_lock(&s_tasks_mutex);
    for (int i = 0; i < MAX_ACTIVE_TASKS; i++) {
        if (s_active_tasks[i] && strcmp(s_active_tasks[i]->req->id, request_id) == 0) {
            s_active_tasks[i]->is_cancelled = true;
            break;
        }
    }
    pthread_mutex_unlock(&s_tasks_mutex);
}

bool http_client_is_running(const char* request_id) {
    if (!request_id) return false;
    bool running = false;
    pthread_mutex_lock(&s_tasks_mutex);
    for (int i = 0; i < MAX_ACTIVE_TASKS; i++) {
        if (s_active_tasks[i] && strcmp(s_active_tasks[i]->req->id, request_id) == 0) {
            running = true;
            break;
        }
    }
    pthread_mutex_unlock(&s_tasks_mutex);
    return running;
}

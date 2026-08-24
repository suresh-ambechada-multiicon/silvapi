#ifndef HTTP_CLIENT_H
#define HTTP_CLIENT_H

#include "silvapi.h"
#include "model.h"

typedef void (*HttpResponseCallback)(const HttpResponse* response, void* user_data);

bool http_client_init(void);
void http_client_shutdown(void);

// Dispatches asynchronous HTTP request on background thread
bool http_client_send(const ApiRequest* req, const Workspace* ws, const Collection* col, const Folder* folder,
                      HttpResponseCallback callback, void* user_data);

void http_client_cancel(const char* request_id);
bool http_client_is_running(const char* request_id);

#endif // HTTP_CLIENT_H

#ifndef IMPORT_EXPORT_H
#define IMPORT_EXPORT_H

#include "base/base.h"
#include "app/model.h"

// Unified Auto-detect Importer
Collection* import_auto_detect(const char* content, char* out_type_detected, size_t out_type_sz);

// Specific Importers
Collection* import_openapi_json(const char* json_content);
Collection* import_postman_json(const char* json_content);
ApiRequest* import_curl_command(const char* curl_cmd);

// Exporters
char* export_curl_command(const ApiRequest* req, const Workspace* ws, const Collection* col, const Folder* folder);

#endif // IMPORT_EXPORT_H

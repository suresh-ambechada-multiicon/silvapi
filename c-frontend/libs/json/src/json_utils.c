#include "json/json_utils.h"
#include <ctype.h>

char* json_format_string(const char* json_raw) {
    if (!json_raw || !*json_raw) return strdup("");
    size_t raw_len = strlen(json_raw);
    // If JSON response is larger than 4MB, return raw to avoid high memory serialization overhead
    if (raw_len > 4 * 1024 * 1024) {
        return strdup(json_raw);
    }
    cJSON* parsed = cJSON_Parse(json_raw);
    if (!parsed) return strdup(json_raw);
    char* formatted = cJSON_Print(parsed);
    cJSON_Delete(parsed);
    if (!formatted) return strdup(json_raw);

    // Replace tabs with 2 spaces for standard API client formatting
    size_t flen = strlen(formatted);
    size_t tab_count = 0;
    for (size_t i = 0; i < flen; i++) {
        if (formatted[i] == '\t') tab_count++;
    }

    if (tab_count == 0) return formatted;

    char* out = malloc(flen + tab_count * 2 + 1);
    size_t out_idx = 0;
    for (size_t i = 0; i < flen; i++) {
        if (formatted[i] == '\t') {
            out[out_idx++] = ' ';
            out[out_idx++] = ' ';
        } else {
            out[out_idx++] = formatted[i];
        }
    }
    out[out_idx] = '\0';
    free(formatted);
    return out;
}

bool json_validate(const char* json_raw) {
    if (!json_raw || !*json_raw) return false;
    cJSON* parsed = cJSON_Parse(json_raw);
    if (!parsed) return false;
    cJSON_Delete(parsed);
    return true;
}

JsonToken* json_tokenize(const char* str, size_t len, size_t* out_token_count) {
    if (!str || len == 0) {
        *out_token_count = 0;
        return NULL;
    }

    size_t cap = 128;
    size_t max_tokens = 100000;
    JsonToken* tokens = malloc(sizeof(JsonToken) * cap);
    size_t count = 0;

    size_t i = 0;

    while (i < len && count < max_tokens) {
        char c = str[i];

        if (isspace((unsigned char)c)) {
            i++;
            continue;
        }

        if (c == '{' || c == '[') {
            if (count >= cap) {
                cap *= 2;
                tokens = realloc(tokens, sizeof(JsonToken) * cap);
            }
            tokens[count++] = (JsonToken){ .start = i, .length = 1, .type = JSON_TOK_PUNCTUATION };
            i++;
            continue;
        }

        if (c == '}' || c == ']' || c == ',' || c == ':') {
            if (count >= cap) {
                cap *= 2;
                tokens = realloc(tokens, sizeof(JsonToken) * cap);
            }
            tokens[count++] = (JsonToken){ .start = i, .length = 1, .type = JSON_TOK_PUNCTUATION };
            i++;
            continue;
        }

        // Strings or Keys
        if (c == '"') {
            size_t start = i;
            i++; // skip opening quote
            while (i < len) {
                if (str[i] == '\\') {
                    i += 2; // skip escape
                    continue;
                }
                if (str[i] == '"') {
                    i++; // skip closing quote
                    break;
                }
                i++;
            }
            size_t str_len = i - start;

            // Check if followed by colon ':' -> it's a key!
            size_t next_non_space = i;
            while (next_non_space < len && isspace((unsigned char)str[next_non_space])) next_non_space++;
            bool is_key = (next_non_space < len && str[next_non_space] == ':');

            if (count >= cap) {
                cap *= 2;
                tokens = realloc(tokens, sizeof(JsonToken) * cap);
            }
            tokens[count++] = (JsonToken){
                .start = start,
                .length = str_len,
                .type = is_key ? JSON_TOK_KEY : JSON_TOK_STRING
            };
            continue;
        }

        // Numbers
        if (c == '-' || (c >= '0' && c <= '9')) {
            size_t start = i;
            while (i < len && (isdigit((unsigned char)str[i]) || str[i] == '.' || str[i] == 'e' || str[i] == 'E' || str[i] == '+' || str[i] == '-')) {
                i++;
            }
            if (count >= cap) {
                cap *= 2;
                tokens = realloc(tokens, sizeof(JsonToken) * cap);
            }
            tokens[count++] = (JsonToken){
                .start = start,
                .length = i - start,
                .type = JSON_TOK_NUMBER
            };
            continue;
        }

        // True / False
        if (strncmp(str + i, "true", 4) == 0) {
            if (count >= cap) { cap *= 2; tokens = realloc(tokens, sizeof(JsonToken) * cap); }
            tokens[count++] = (JsonToken){ .start = i, .length = 4, .type = JSON_TOK_BOOLEAN };
            i += 4;
            continue;
        }
        if (strncmp(str + i, "false", 5) == 0) {
            if (count >= cap) { cap *= 2; tokens = realloc(tokens, sizeof(JsonToken) * cap); }
            tokens[count++] = (JsonToken){ .start = i, .length = 5, .type = JSON_TOK_BOOLEAN };
            i += 5;
            continue;
        }

        // Null
        if (strncmp(str + i, "null", 4) == 0) {
            if (count >= cap) { cap *= 2; tokens = realloc(tokens, sizeof(JsonToken) * cap); }
            tokens[count++] = (JsonToken){ .start = i, .length = 4, .type = JSON_TOK_NULL };
            i += 4;
            continue;
        }

        // Comments //
        if (str[i] == '/' && i + 1 < len && str[i + 1] == '/') {
            size_t start = i;
            while (i < len && str[i] != '\n') i++;
            if (count >= cap) { cap *= 2; tokens = realloc(tokens, sizeof(JsonToken) * cap); }
            tokens[count++] = (JsonToken){ .start = start, .length = i - start, .type = JSON_TOK_COMMENT };
            continue;
        }

        i++;
    }

    *out_token_count = count;
    return tokens;
}

static const char* json_type_name(cJSON* val) {
    if (!val) return "null";
    if (cJSON_IsString(val)) return "string";
    if (cJSON_IsNumber(val)) return "number";
    if (cJSON_IsBool(val)) return "boolean";
    if (cJSON_IsArray(val)) return "array";
    if (cJSON_IsObject(val)) return "object";
    if (cJSON_IsNull(val)) return "null";
    return "unknown";
}

static char* json_value_to_string(cJSON* val) {
    if (!val) return strdup("");
    if (cJSON_IsString(val)) return strdup(val->valuestring ? val->valuestring : "");
    if (cJSON_IsNumber(val)) {
        char buf[64];
        if (val->valuedouble == (double)val->valueint) {
            snprintf(buf, sizeof(buf), "%d", val->valueint);
        } else {
            snprintf(buf, sizeof(buf), "%g", val->valuedouble);
        }
        return strdup(buf);
    }
    if (cJSON_IsBool(val)) return strdup(cJSON_IsTrue(val) ? "true" : "false");
    if (cJSON_IsNull(val)) return strdup("null");
    char* printed = cJSON_PrintUnformatted(val);
    return printed ? printed : strdup("");
}

// Convert JSON object/array to Vertical Table format (Property | Type | Value)
JsonTable* json_table_from_string(const char* json_str) {
    if (!json_str || !*json_str) return NULL;
    cJSON* root = cJSON_Parse(json_str);
    if (!root) return NULL;

    JsonTable* table = calloc(1, sizeof(JsonTable));

    if (cJSON_IsObject(root)) {
        // Vertical Property Inspector for JSON Object
        table->column_count = 3;
        table->columns = malloc(sizeof(char*) * 3);
        table->columns[0] = strdup("Property / Field");
        table->columns[1] = strdup("Type");
        table->columns[2] = strdup("Value");

        size_t row_count = 0;
        cJSON* child = root->child;
        while (child) {
            row_count++;
            child = child->next;
        }

        table->row_count = row_count;
        table->rows = malloc(sizeof(char**) * row_count);

        child = root->child;
        size_t r = 0;
        while (child && r < row_count) {
            table->rows[r] = malloc(sizeof(char*) * 3);
            table->rows[r][0] = strdup(child->string ? child->string : "");
            table->rows[r][1] = strdup(json_type_name(child));
            table->rows[r][2] = json_value_to_string(child);
            child = child->next;
            r++;
        }
    } else if (cJSON_IsArray(root)) {
        // Vertical Table for JSON Array: [#] Index | Property | Type | Value
        table->column_count = 4;
        table->columns = malloc(sizeof(char*) * 4);
        table->columns[0] = strdup("# Index");
        table->columns[1] = strdup("Property / Key");
        table->columns[2] = strdup("Type");
        table->columns[3] = strdup("Value");

        // Count total rows across all items
        int arr_size = cJSON_GetArraySize(root);
        size_t total_rows = 0;
        for (int i = 0; i < arr_size && total_rows < 5000; i++) {
            cJSON* item = cJSON_GetArrayItem(root, i);
            if (cJSON_IsObject(item)) {
                cJSON* child = item->child;
                while (child && total_rows < 5000) {
                    total_rows++;
                    child = child->next;
                }
            } else {
                total_rows++;
            }
        }

        table->row_count = total_rows;
        table->rows = malloc(sizeof(char**) * total_rows);

        size_t r = 0;
        for (int i = 0; i < arr_size; i++) {
            cJSON* item = cJSON_GetArrayItem(root, i);
            char idx_str[32];
            snprintf(idx_str, sizeof(idx_str), "[%d]", i);

            if (cJSON_IsObject(item)) {
                cJSON* child = item->child;
                while (child && r < total_rows) {
                    table->rows[r] = malloc(sizeof(char*) * 4);
                    table->rows[r][0] = strdup(idx_str);
                    table->rows[r][1] = strdup(child->string ? child->string : "");
                    table->rows[r][2] = strdup(json_type_name(child));
                    table->rows[r][3] = json_value_to_string(child);
                    child = child->next;
                    r++;
                }
            } else {
                if (r < total_rows) {
                    table->rows[r] = malloc(sizeof(char*) * 4);
                    table->rows[r][0] = strdup(idx_str);
                    table->rows[r][1] = strdup("");
                    table->rows[r][2] = strdup(json_type_name(item));
                    table->rows[r][3] = json_value_to_string(item);
                    r++;
                }
            }
        }
    } else {
        cJSON_Delete(root);
        free(table);
        return NULL;
    }

    cJSON_Delete(root);
    return table;
}

void json_table_free(JsonTable* table) {
    if (!table) return;
    if (table->columns) {
        for (size_t i = 0; i < table->column_count; i++) {
            if (table->columns[i]) free(table->columns[i]);
        }
        free(table->columns);
    }
    if (table->rows) {
        for (size_t r = 0; r < table->row_count; r++) {
            if (table->rows[r]) {
                for (size_t c = 0; c < table->column_count; c++) {
                    if (table->rows[r][c]) free(table->rows[r][c]);
                }
                free(table->rows[r]);
            }
        }
        free(table->rows);
    }
    free(table);
}

char* json_table_to_csv(const JsonTable* table) {
    if (!table || table->column_count == 0) return strdup("");

    size_t cap = 1024;
    char* csv = malloc(cap);
    size_t len = 0;
    csv[0] = '\0';

    // Header row
    for (size_t c = 0; c < table->column_count; c++) {
        size_t clen = strlen(table->columns[c]);
        while (len + clen + 4 >= cap) {
            cap *= 2;
            csv = realloc(csv, cap);
        }
        if (c > 0) csv[len++] = ',';
        strcpy(csv + len, table->columns[c]);
        len += clen;
    }
    csv[len++] = '\n';
    csv[len] = '\0';

    // Data rows
    for (size_t r = 0; r < table->row_count; r++) {
        for (size_t c = 0; c < table->column_count; c++) {
            const char* val = table->rows[r][c];
            size_t vlen = strlen(val);
            while (len + vlen + 16 >= cap) {
                cap *= 2;
                csv = realloc(csv, cap);
            }
            if (c > 0) csv[len++] = ',';
            if (strchr(val, ',') || strchr(val, '"') || strchr(val, '\n')) {
                csv[len++] = '"';
                for (size_t k = 0; k < vlen; k++) {
                    if (val[k] == '"') csv[len++] = '"';
                    csv[len++] = val[k];
                }
                csv[len++] = '"';
            } else {
                strcpy(csv + len, val);
                len += vlen;
            }
        }
        csv[len++] = '\n';
        csv[len] = '\0';
    }

    return csv;
}

// ---------------------------------------------------------------------------
// Single-line, non-allocating tokenizer.
//
// A key is a string immediately followed (past whitespace) by ':'. That
// lookahead is the only reason keys and plain strings can be told apart without
// tracking brace depth across the whole document.
// ---------------------------------------------------------------------------

size_t json_tokenize_line(const char* line, size_t len,
                          JsonLineToken* out, size_t max) {
    if (!line || !out || max == 0) return 0;

    size_t n = 0;
    size_t i = 0;

    while (i < len && n < max) {
        unsigned char c = (unsigned char)line[i];

        if (c == ' ' || c == '\t' || c == '\r') {
            i++;
            continue;
        }

        if (c == '"') {
            size_t start = i;
            i++;
            while (i < len) {
                if (line[i] == '\\' && i + 1 < len) { i += 2; continue; }
                if (line[i] == '"') { i++; break; }
                i++;
            }

            size_t j = i;
            while (j < len && (line[j] == ' ' || line[j] == '\t')) j++;
            bool is_key = (j < len && line[j] == ':');

            out[n].start = (uint32_t)start;
            out[n].length = (uint32_t)(i - start);
            out[n].type = (uint8_t)(is_key ? JSON_TOK_KEY : JSON_TOK_STRING);
            n++;
            continue;
        }

        if ((c >= '0' && c <= '9') || c == '-' ||
            (c == '+' && i + 1 < len && isdigit((unsigned char)line[i + 1]))) {
            size_t start = i;
            i++;
            while (i < len && (isdigit((unsigned char)line[i]) || line[i] == '.' ||
                               line[i] == 'e' || line[i] == 'E' ||
                               line[i] == '+' || line[i] == '-')) {
                i++;
            }
            out[n].start = (uint32_t)start;
            out[n].length = (uint32_t)(i - start);
            out[n].type = (uint8_t)JSON_TOK_NUMBER;
            n++;
            continue;
        }

        if (isalpha(c)) {
            size_t start = i;
            while (i < len && isalpha((unsigned char)line[i])) i++;
            size_t wl = i - start;

            uint8_t type = (uint8_t)JSON_TOK_NONE;
            if ((wl == 4 && strncmp(line + start, "true", 4) == 0) ||
                (wl == 5 && strncmp(line + start, "false", 5) == 0)) {
                type = (uint8_t)JSON_TOK_BOOLEAN;
            } else if (wl == 4 && strncmp(line + start, "null", 4) == 0) {
                type = (uint8_t)JSON_TOK_NULL;
            }
            if (type != (uint8_t)JSON_TOK_NONE) {
                out[n].start = (uint32_t)start;
                out[n].length = (uint32_t)wl;
                out[n].type = type;
                n++;
            }
            continue;
        }

        if (c == '{' || c == '}' || c == '[' || c == ']' || c == ':' || c == ',') {
            out[n].start = (uint32_t)i;
            out[n].length = 1;
            out[n].type = (uint8_t)JSON_TOK_PUNCTUATION;
            n++;
            i++;
            continue;
        }

        i++;
    }
    return n;
}

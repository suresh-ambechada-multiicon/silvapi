#ifndef JSON_UTILS_H
#define JSON_UTILS_H

#include "base/base.h"
#include "cJSON.h"

typedef enum {
    JSON_TOK_NONE = 0,
    JSON_TOK_KEY,
    JSON_TOK_STRING,
    JSON_TOK_NUMBER,
    JSON_TOK_BOOLEAN,
    JSON_TOK_NULL,
    JSON_TOK_PUNCTUATION,
    JSON_TOK_COMMENT
} JsonTokenType;

typedef struct {
    size_t start;
    size_t length;
    JsonTokenType type;
} JsonToken;

char* json_format_string(const char* json_raw);
bool json_validate(const char* json_raw);

// Tokenize JSON for syntax-highlighted rendering in code editor
JsonToken* json_tokenize(const char* json_str, size_t len, size_t* token_count);

// Non-allocating, single-line variant for a syntax-highlighting editor, which
// tokenizes only the lines it draws and does so every frame. The allocating
// whole-document form above is unusable there.
//
// Per-line is sound for JSON: a string literal cannot legally span a raw
// newline, so no lexer state needs to carry across lines.
typedef struct {
    uint32_t start;
    uint32_t length;
    uint8_t  type;    // JsonTokenType
} JsonLineToken;

size_t json_tokenize_line(const char* line, size_t len,
                          JsonLineToken* out, size_t max);

// Table view helper for JSON array of objects
typedef struct {
    char** columns;
    size_t column_count;
    char*** rows; // rows[row_idx][col_idx]
    size_t row_count;
} JsonTable;

JsonTable* json_table_from_string(const char* json_str);
void json_table_free(JsonTable* table);
char* json_table_to_csv(const JsonTable* table);

#endif // JSON_UTILS_H

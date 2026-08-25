#ifndef FREDBUF_H
#define FREDBUF_H

#include <stddef.h>
#include <stdbool.h>
#include <stdint.h>

// FredBuf: High-performance gap buffer and line index text structure
// inspired by cdacamar/fredbuf and piece table architectures.

typedef struct {
    size_t start;
    size_t length;
} TextRange;

typedef struct {
    size_t line;   // 0-indexed line
    size_t col;    // 0-indexed UTF-8 column
    size_t offset; // byte offset in document
} TextCursor;

typedef struct UndoAction {
    bool is_insert;       // true = insert, false = delete
    size_t offset;
    char* text;
    size_t text_len;
    struct UndoAction* next;
} UndoAction;

typedef struct {
    char* buffer;         // allocated memory
    size_t capacity;      // total allocated capacity
    size_t gap_start;     // start of the gap
    size_t gap_end;       // end of the gap
    
    // Line index cache
    size_t* line_starts;  // dynamic array of byte offsets for each line
    size_t line_count;
    size_t line_capacity;
    bool line_starts_dirty;

    // Undo / Redo stacks
    UndoAction* undo_stack;
    UndoAction* redo_stack;
    bool in_undo_redo;
} FredBuffer;

// Lifecycle
FredBuffer* fredbuf_create(size_t initial_capacity);
FredBuffer* fredbuf_create_from_string(const char* initial_text);
void fredbuf_free(FredBuffer* buf);
void fredbuf_clear(FredBuffer* buf);

// Inspection
size_t fredbuf_length(const FredBuffer* buf);
size_t fredbuf_line_count(FredBuffer* buf);
char fredbuf_char_at(const FredBuffer* buf, size_t offset);
char* fredbuf_to_string(const FredBuffer* buf);
void fredbuf_copy_range(const FredBuffer* buf, size_t offset, size_t len, char* out_str);

// Line access
void fredbuf_get_line(FredBuffer* buf, size_t line_idx, size_t* out_start, size_t* out_len);
char* fredbuf_get_line_string(FredBuffer* buf, size_t line_idx);
void fredbuf_offset_to_line_col(FredBuffer* buf, size_t offset, size_t* out_line, size_t* out_col);
size_t fredbuf_line_col_to_offset(FredBuffer* buf, size_t line, size_t col);

// Mutations
void fredbuf_insert_char(FredBuffer* buf, size_t offset, char c);
void fredbuf_insert_string(FredBuffer* buf, size_t offset, const char* str, size_t len);
void fredbuf_delete_range(FredBuffer* buf, size_t offset, size_t len);
void fredbuf_replace_all(FredBuffer* buf, const char* new_text);

// Undo / Redo
void fredbuf_undo(FredBuffer* buf, size_t* out_cursor);
void fredbuf_redo(FredBuffer* buf, size_t* out_cursor);

// Fast Search (Boyer-Moore-Horspool algorithm)
bool fredbuf_find_forward(const FredBuffer* buf, const char* query, size_t query_len, size_t start_offset, bool case_sensitive, TextRange* out_match);
bool fredbuf_find_backward(const FredBuffer* buf, const char* query, size_t query_len, size_t start_offset, bool case_sensitive, TextRange* out_match);
bool fredbuf_find_all(const FredBuffer* buf, const char* query, size_t query_len, bool case_sensitive, TextRange** out_ranges, size_t* out_count);
size_t fredbuf_search_text(const char* text, size_t text_len, const char* query, size_t query_len, bool case_sensitive, TextRange** out_ranges, size_t* out_count);
bool fredbuf_replace_range(FredBuffer* buf, size_t offset, size_t len, const char* replacement, size_t repl_len);
size_t fredbuf_replace_all_matches(FredBuffer* buf, const char* query, size_t query_len, const char* replacement, size_t repl_len, bool case_sensitive);

#endif // FREDBUF_H

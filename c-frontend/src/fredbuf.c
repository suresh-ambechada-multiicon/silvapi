#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif
#include "fredbuf.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#define DEFAULT_CAPACITY 1024

static void free_undo_action(UndoAction* act) {
    if (!act) return;
    if (act->text) free(act->text);
    free(act);
}

static void clear_undo_stack(UndoAction** stack) {
    UndoAction* cur = *stack;
    while (cur) {
        UndoAction* next = cur->next;
        free_undo_action(cur);
        cur = next;
    }
    *stack = NULL;
}

static void push_undo(FredBuffer* buf, bool is_insert, size_t offset, const char* text, size_t len) {
    if (buf->in_undo_redo) return;
    UndoAction* act = calloc(1, sizeof(UndoAction));
    act->is_insert = is_insert;
    act->offset = offset;
    act->text_len = len;
    act->text = malloc(len + 1);
    memcpy(act->text, text, len);
    act->text[len] = '\0';

    act->next = buf->undo_stack;
    buf->undo_stack = act;

    // Clear redo on new action
    clear_undo_stack(&buf->redo_stack);
}

FredBuffer* fredbuf_create(size_t initial_capacity) {
    if (initial_capacity < 64) initial_capacity = DEFAULT_CAPACITY;
    FredBuffer* buf = calloc(1, sizeof(FredBuffer));
    buf->capacity = initial_capacity;
    buf->buffer = malloc(buf->capacity);
    buf->gap_start = 0;
    buf->gap_end = buf->capacity;

    buf->line_capacity = 64;
    buf->line_starts = malloc(sizeof(size_t) * buf->line_capacity);
    buf->line_starts[0] = 0;
    buf->line_count = 1;
    buf->line_starts_dirty = false;

    return buf;
}

FredBuffer* fredbuf_create_from_string(const char* initial_text) {
    size_t len = initial_text ? strlen(initial_text) : 0;
    size_t cap = len > 0 ? (len * 2 + 64) : DEFAULT_CAPACITY;
    FredBuffer* buf = fredbuf_create(cap);
    if (len > 0) {
        fredbuf_insert_string(buf, 0, initial_text, len);
    }
    return buf;
}

void fredbuf_free(FredBuffer* buf) {
    if (!buf) return;
    if (buf->buffer) free(buf->buffer);
    if (buf->line_starts) free(buf->line_starts);
    clear_undo_stack(&buf->undo_stack);
    clear_undo_stack(&buf->redo_stack);
    free(buf);
}

void fredbuf_clear(FredBuffer* buf) {
    if (!buf) return;
    buf->gap_start = 0;
    buf->gap_end = buf->capacity;
    buf->line_count = 1;
    buf->line_starts[0] = 0;
    buf->line_starts_dirty = false;
    clear_undo_stack(&buf->undo_stack);
    clear_undo_stack(&buf->redo_stack);
}

size_t fredbuf_length(const FredBuffer* buf) {
    if (!buf) return 0;
    return buf->capacity - (buf->gap_end - buf->gap_start);
}

static void move_gap_to(FredBuffer* buf, size_t offset) {
    if (offset == buf->gap_start) return;

    if (offset < buf->gap_start) {
        size_t dist = buf->gap_start - offset;
        memmove(buf->buffer + buf->gap_end - dist, buf->buffer + offset, dist);
        buf->gap_start -= dist;
        buf->gap_end -= dist;
    } else {
        size_t dist = offset - buf->gap_start;
        memmove(buf->buffer + buf->gap_start, buf->buffer + buf->gap_end, dist);
        buf->gap_start += dist;
        buf->gap_end += dist;
    }
}

static void grow_buffer(FredBuffer* buf, size_t needed) {
    size_t gap_size = buf->gap_end - buf->gap_start;
    if (gap_size >= needed) return;

    size_t new_capacity = (buf->capacity * 2) + needed + 128;
    char* new_buffer = malloc(new_capacity);

    // Copy head
    memcpy(new_buffer, buf->buffer, buf->gap_start);
    // Copy tail
    size_t tail_len = buf->capacity - buf->gap_end;
    size_t new_gap_end = new_capacity - tail_len;
    memcpy(new_buffer + new_gap_end, buf->buffer + buf->gap_end, tail_len);

    free(buf->buffer);
    buf->buffer = new_buffer;
    buf->capacity = new_capacity;
    buf->gap_end = new_gap_end;
}

static void rebuild_line_starts(FredBuffer* buf) {
    if (!buf->line_starts_dirty) return;

    buf->line_count = 0;
    size_t total_len = fredbuf_length(buf);

    if (buf->line_capacity < 64) {
        buf->line_capacity = 64;
        buf->line_starts = realloc(buf->line_starts, sizeof(size_t) * buf->line_capacity);
    }
    buf->line_starts[buf->line_count++] = 0;

    for (size_t i = 0; i < total_len; i++) {
        char c = fredbuf_char_at(buf, i);
        if (c == '\n') {
            if (buf->line_count >= buf->line_capacity) {
                buf->line_capacity *= 2;
                buf->line_starts = realloc(buf->line_starts, sizeof(size_t) * buf->line_capacity);
            }
            buf->line_starts[buf->line_count++] = i + 1;
        }
    }

    buf->line_starts_dirty = false;
}

char fredbuf_char_at(const FredBuffer* buf, size_t offset) {
    if (!buf || offset >= fredbuf_length(buf)) return '\0';
    if (offset < buf->gap_start) {
        return buf->buffer[offset];
    } else {
        return buf->buffer[offset + (buf->gap_end - buf->gap_start)];
    }
}

char* fredbuf_to_string(const FredBuffer* buf) {
    if (!buf) return strdup("");
    size_t len = fredbuf_length(buf);
    char* str = malloc(len + 1);
    if (buf->gap_start > 0) {
        memcpy(str, buf->buffer, buf->gap_start);
    }
    size_t tail_len = buf->capacity - buf->gap_end;
    if (tail_len > 0) {
        memcpy(str + buf->gap_start, buf->buffer + buf->gap_end, tail_len);
    }
    str[len] = '\0';
    return str;
}

void fredbuf_copy_range(const FredBuffer* buf, size_t offset, size_t len, char* out_str) {
    if (!buf || !out_str || len == 0) return;
    size_t total_len = fredbuf_length(buf);
    if (offset >= total_len) {
        out_str[0] = '\0';
        return;
    }
    if (offset + len > total_len) len = total_len - offset;

    for (size_t i = 0; i < len; i++) {
        out_str[i] = fredbuf_char_at(buf, offset + i);
    }
    out_str[len] = '\0';
}

void fredbuf_insert_char(FredBuffer* buf, size_t offset, char c) {
    fredbuf_insert_string(buf, offset, &c, 1);
}

void fredbuf_insert_string(FredBuffer* buf, size_t offset, const char* str, size_t len) {
    if (!buf || !str || len == 0) return;
    size_t total_len = fredbuf_length(buf);
    if (offset > total_len) offset = total_len;

    grow_buffer(buf, len);
    move_gap_to(buf, offset);

    memcpy(buf->buffer + buf->gap_start, str, len);
    buf->gap_start += len;
    buf->line_starts_dirty = true;

    push_undo(buf, true, offset, str, len);
}

void fredbuf_delete_range(FredBuffer* buf, size_t offset, size_t len) {
    if (!buf || len == 0) return;
    size_t total_len = fredbuf_length(buf);
    if (offset >= total_len) return;
    if (offset + len > total_len) len = total_len - offset;

    // Save deleted text for undo
    char* deleted_text = malloc(len + 1);
    fredbuf_copy_range(buf, offset, len, deleted_text);

    move_gap_to(buf, offset);
    buf->gap_end += len;
    buf->line_starts_dirty = true;

    push_undo(buf, false, offset, deleted_text, len);
    free(deleted_text);
}

void fredbuf_replace_all(FredBuffer* buf, const char* new_text) {
    if (!buf) return;
    fredbuf_clear(buf);
    if (new_text && *new_text) {
        fredbuf_insert_string(buf, 0, new_text, strlen(new_text));
    }
}

size_t fredbuf_line_count(FredBuffer* buf) {
    if (!buf) return 1;
    rebuild_line_starts(buf);
    return buf->line_count;
}

void fredbuf_get_line(FredBuffer* buf, size_t line_idx, size_t* out_start, size_t* out_len) {
    if (!buf) {
        if (out_start) *out_start = 0;
        if (out_len) *out_len = 0;
        return;
    }
    rebuild_line_starts(buf);
    if (line_idx >= buf->line_count) {
        if (out_start) *out_start = fredbuf_length(buf);
        if (out_len) *out_len = 0;
        return;
    }

    size_t start = buf->line_starts[line_idx];
    size_t total_len = fredbuf_length(buf);
    size_t next_start = (line_idx + 1 < buf->line_count) ? buf->line_starts[line_idx + 1] : total_len;
    size_t len = next_start - start;

    // Trim trailing \r or \n
    while (len > 0) {
        char c = fredbuf_char_at(buf, start + len - 1);
        if (c == '\r' || c == '\n') len--;
        else break;
    }

    if (out_start) *out_start = start;
    if (out_len) *out_len = len;
}

char* fredbuf_get_line_string(FredBuffer* buf, size_t line_idx) {
    size_t start, len;
    fredbuf_get_line(buf, line_idx, &start, &len);
    char* str = malloc(len + 1);
    fredbuf_copy_range(buf, start, len, str);
    return str;
}

void fredbuf_offset_to_line_col(FredBuffer* buf, size_t offset, size_t* out_line, size_t* out_col) {
    if (!buf) {
        if (out_line) *out_line = 0;
        if (out_col) *out_col = 0;
        return;
    }
    rebuild_line_starts(buf);
    size_t total_len = fredbuf_length(buf);
    if (offset > total_len) offset = total_len;

    // Binary search in line_starts
    size_t low = 0;
    size_t high = buf->line_count - 1;
    size_t line = 0;

    while (low <= high) {
        size_t mid = (low + high) / 2;
        if (buf->line_starts[mid] <= offset) {
            line = mid;
            if (mid + 1 == buf->line_count || buf->line_starts[mid + 1] > offset) break;
            low = mid + 1;
        } else {
            if (mid == 0) break;
            high = mid - 1;
        }
    }

    if (out_line) *out_line = line;
    if (out_col) *out_col = offset - buf->line_starts[line];
}

size_t fredbuf_line_col_to_offset(FredBuffer* buf, size_t line, size_t col) {
    if (!buf) return 0;
    rebuild_line_starts(buf);
    if (line >= buf->line_count) line = buf->line_count - 1;

    size_t line_start = buf->line_starts[line];
    size_t total_len = fredbuf_length(buf);
    size_t next_start = (line + 1 < buf->line_count) ? buf->line_starts[line + 1] : total_len;
    size_t line_len = next_start - line_start;

    if (col > line_len) col = line_len;
    return line_start + col;
}

void fredbuf_undo(FredBuffer* buf, size_t* out_cursor) {
    if (!buf || !buf->undo_stack) return;
    UndoAction* act = buf->undo_stack;
    buf->undo_stack = act->next;

    buf->in_undo_redo = true;
    if (act->is_insert) {
        // Reverse insert -> delete
        fredbuf_delete_range(buf, act->offset, act->text_len);
        if (out_cursor) *out_cursor = act->offset;
    } else {
        // Reverse delete -> insert
        fredbuf_insert_string(buf, act->offset, act->text, act->text_len);
        if (out_cursor) *out_cursor = act->offset + act->text_len;
    }
    buf->in_undo_redo = false;

    // Push to redo
    act->next = buf->redo_stack;
    buf->redo_stack = act;
}

void fredbuf_redo(FredBuffer* buf, size_t* out_cursor) {
    if (!buf || !buf->redo_stack) return;
    UndoAction* act = buf->redo_stack;
    buf->redo_stack = act->next;

    buf->in_undo_redo = true;
    if (act->is_insert) {
        fredbuf_insert_string(buf, act->offset, act->text, act->text_len);
        if (out_cursor) *out_cursor = act->offset + act->text_len;
    } else {
        fredbuf_delete_range(buf, act->offset, act->text_len);
        if (out_cursor) *out_cursor = act->offset;
    }
    buf->in_undo_redo = false;

    // Push to undo
    act->next = buf->undo_stack;
    buf->undo_stack = act;
}

#include <ctype.h>

static inline bool fred_char_eq(char a, char b, bool case_sensitive) {
    if (case_sensitive) return a == b;
    return tolower((unsigned char)a) == tolower((unsigned char)b);
}

// Build Horspool skip table
static void fred_build_skip_table(const char* query, size_t m, bool case_sensitive, size_t skip[256]) {
    for (int i = 0; i < 256; i++) {
        skip[i] = m;
    }
    for (size_t i = 0; i < m - 1; i++) {
        unsigned char c = (unsigned char)query[i];
        if (case_sensitive) {
            skip[c] = m - 1 - i;
        } else {
            skip[tolower(c)] = m - 1 - i;
            skip[toupper(c)] = m - 1 - i;
        }
    }
}

// Fast Boyer-Moore-Horspool search in contiguous memory
size_t fredbuf_search_text(const char* text, size_t text_len, const char* query, size_t query_len, bool case_sensitive, TextRange** out_ranges, size_t* out_count) {
    if (out_count) *out_count = 0;
    if (out_ranges) *out_ranges = NULL;
    if (!text || text_len == 0 || !query || query_len == 0 || text_len < query_len) {
        return 0;
    }

    size_t skip[256];
    fred_build_skip_table(query, query_len, case_sensitive, skip);

    size_t count = 0;
    size_t cap = 32;
    TextRange* ranges = out_ranges ? malloc(sizeof(TextRange) * cap) : NULL;

    size_t k = query_len - 1;
    while (k < text_len) {
        size_t i = query_len - 1;
        size_t j = k;
        while (fred_char_eq(text[j], query[i], case_sensitive)) {
            if (i == 0) {
                // Match found at j
                if (ranges) {
                    if (count >= cap) {
                        cap *= 2;
                        ranges = realloc(ranges, sizeof(TextRange) * cap);
                    }
                    ranges[count].start = j;
                    ranges[count].length = query_len;
                }
                count++;
                break;
            }
            i--;
            j--;
        }
        unsigned char last_c = (unsigned char)text[k];
        size_t step = case_sensitive ? skip[last_c] : skip[tolower(last_c)];
        if (step == 0) step = 1;
        k += step;
    }

    if (out_count) *out_count = count;
    if (out_ranges) *out_ranges = ranges;
    return count;
}

bool fredbuf_find_all(const FredBuffer* buf, const char* query, size_t query_len, bool case_sensitive, TextRange** out_ranges, size_t* out_count) {
    if (!buf || !query || query_len == 0) {
        if (out_count) *out_count = 0;
        if (out_ranges) *out_ranges = NULL;
        return false;
    }

    char* text = fredbuf_to_string(buf);
    size_t len = fredbuf_length(buf);
    size_t count = fredbuf_search_text(text, len, query, query_len, case_sensitive, out_ranges, out_count);
    free(text);
    return count > 0;
}

bool fredbuf_find_forward(const FredBuffer* buf, const char* query, size_t query_len, size_t start_offset, bool case_sensitive, TextRange* out_match) {
    if (!buf || !query || query_len == 0) return false;
    char* text = fredbuf_to_string(buf);
    size_t len = fredbuf_length(buf);
    if (len < query_len) {
        free(text);
        return false;
    }

    if (start_offset > len - query_len) start_offset = 0; // Wrap around if starting past end

    size_t skip[256];
    fred_build_skip_table(query, query_len, case_sensitive, skip);

    // Search from start_offset to end
    size_t k = start_offset + query_len - 1;
    while (k < len) {
        size_t i = query_len - 1;
        size_t j = k;
        while (fred_char_eq(text[j], query[i], case_sensitive)) {
            if (i == 0) {
                if (out_match) {
                    out_match->start = j;
                    out_match->length = query_len;
                }
                free(text);
                return true;
            }
            i--;
            j--;
        }
        unsigned char last_c = (unsigned char)text[k];
        size_t step = case_sensitive ? skip[last_c] : skip[tolower(last_c)];
        if (step == 0) step = 1;
        k += step;
    }

    // If not found and start_offset > 0, wrap around to search from 0 to start_offset
    if (start_offset > 0) {
        k = query_len - 1;
        size_t limit = start_offset + query_len - 1;
        if (limit > len) limit = len;
        while (k < limit) {
            size_t i = query_len - 1;
            size_t j = k;
            while (fred_char_eq(text[j], query[i], case_sensitive)) {
                if (i == 0) {
                    if (out_match) {
                        out_match->start = j;
                        out_match->length = query_len;
                    }
                    free(text);
                    return true;
                }
                i--;
                j--;
            }
            unsigned char last_c = (unsigned char)text[k];
            size_t step = case_sensitive ? skip[last_c] : skip[tolower(last_c)];
            if (step == 0) step = 1;
            k += step;
        }
    }

    free(text);
    return false;
}

bool fredbuf_find_backward(const FredBuffer* buf, const char* query, size_t query_len, size_t start_offset, bool case_sensitive, TextRange* out_match) {
    if (!buf || !query || query_len == 0) return false;
    TextRange* ranges = NULL;
    size_t count = 0;
    if (!fredbuf_find_all(buf, query, query_len, case_sensitive, &ranges, &count) || count == 0) {
        return false;
    }

    // Find the match immediately before start_offset
    int found_idx = -1;
    for (int i = (int)count - 1; i >= 0; i--) {
        if (ranges[i].start < start_offset) {
            found_idx = i;
            break;
        }
    }
    // If none before start_offset, wrap to last match
    if (found_idx < 0) {
        found_idx = (int)count - 1;
    }

    if (out_match) {
        *out_match = ranges[found_idx];
    }
    free(ranges);
    return true;
}

bool fredbuf_replace_range(FredBuffer* buf, size_t offset, size_t len, const char* replacement, size_t repl_len) {
    if (!buf) return false;
    fredbuf_delete_range(buf, offset, len);
    if (replacement && repl_len > 0) {
        fredbuf_insert_string(buf, offset, replacement, repl_len);
    }
    return true;
}

size_t fredbuf_replace_all_matches(FredBuffer* buf, const char* query, size_t query_len, const char* replacement, size_t repl_len, bool case_sensitive) {
    if (!buf || !query || query_len == 0) return 0;
    TextRange* ranges = NULL;
    size_t count = 0;
    if (!fredbuf_find_all(buf, query, query_len, case_sensitive, &ranges, &count) || count == 0) {
        return 0;
    }

    // Replace from back to front so offsets remain valid
    for (int i = (int)count - 1; i >= 0; i--) {
        fredbuf_replace_range(buf, ranges[i].start, ranges[i].length, replacement, repl_len);
    }
    free(ranges);
    return count;
}

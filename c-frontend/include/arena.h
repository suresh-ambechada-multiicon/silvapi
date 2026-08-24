#ifndef ARENA_H
#define ARENA_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdarg.h>

// Arena memory allocator inspired by RAD Debugger (raddbg)
// Provides virtual memory reserve/commit, scratch arenas, and String8 utilities.

#define ARENA_DEFAULT_RESERVE (64 * 1024 * 1024) // 64 MB virtual address space
#define ARENA_COMMIT_BLOCK_SIZE (64 * 1024)       // 64 KB commit chunk

typedef struct Arena {
    uint8_t* memory;
    size_t capacity;     // Total reserved virtual address space
    size_t pos;          // Current allocation offset
    size_t commit_pos;   // Currently committed physical memory
    size_t align;        // Default alignment (default: 8 or 16)
} Arena;

typedef struct Temp {
    Arena* arena;
    size_t pos;
} Temp;

// Arena Lifecycle
Arena* arena_create(size_t reserve_capacity);
void   arena_destroy(Arena* arena);
void   arena_clear(Arena* arena);

// Core Allocation
void*  arena_push(Arena* arena, size_t size);
void*  arena_push_align(Arena* arena, size_t size, size_t align);
void*  arena_push_zero(Arena* arena, size_t size);
void*  arena_push_copy(Arena* arena, const void* src, size_t size);
void   arena_pop_to(Arena* arena, size_t pos);
void   arena_pop(Arena* arena, size_t size);

// Macros for typed allocations
#define arena_push_struct(arena, type) ((type*)arena_push_zero((arena), sizeof(type)))
#define arena_push_array(arena, type, count) ((type*)arena_push((arena), sizeof(type) * (count)))
#define arena_push_array_zero(arena, type, count) ((type*)arena_push_zero((arena), sizeof(type) * (count)))

// Scoped Temp / Scratch
Temp   temp_begin(Arena* arena);
void   temp_end(Temp temp);

// Global thread-safe scratch arena pool (conflict-avoiding)
Temp   scratch_begin(Arena** conflicts, size_t conflict_count);
void   scratch_end(Temp scratch);

// -----------------------------------------------------------------------------
// String8: High-performance string slice (pointer + length)
// -----------------------------------------------------------------------------

typedef struct String8 {
    uint8_t* str;
    size_t size;
} String8;

#define str8_lit(s) (String8){(uint8_t*)(s), sizeof(s) - 1}
#define str8_expand(s) (int)(s).size, (char*)(s).str
#define str8_empty (String8){NULL, 0}

String8 str8(const uint8_t* str, size_t size);
String8 str8_from_cstr(const char* cstr);
String8 str8_push_copy(Arena* arena, String8 s);
String8 str8_push_cstr(Arena* arena, const char* cstr);
String8 str8_pushf(Arena* arena, const char* fmt, ...);
String8 str8_pushfv(Arena* arena, const char* fmt, va_list args);
char*   str8_to_cstr(Arena* arena, String8 s);
bool    str8_equals(String8 a, String8 b);
bool    str8_equals_ci(String8 a, String8 b);
String8 str8_substr(String8 s, size_t start, size_t length);
String8 str8_concat(Arena* arena, String8 a, String8 b);
String8 str8_trim(String8 s);

#endif // ARENA_H

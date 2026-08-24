#include "arena.h"
#include <sys/mman.h>
#include <unistd.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <ctype.h>

#define DEFAULT_ALIGNMENT 8

static size_t align_forward(size_t ptr, size_t align) {
    size_t remainder = ptr % align;
    if (remainder != 0) {
        ptr += (align - remainder);
    }
    return ptr;
}

static size_t get_os_page_size(void) {
    static size_t page_size = 0;
    if (page_size == 0) {
        long sz = sysconf(_SC_PAGESIZE);
        page_size = (sz > 0) ? (size_t)sz : 4096;
    }
    return page_size;
}

Arena* arena_create(size_t reserve_capacity) {
    if (reserve_capacity == 0) {
        reserve_capacity = ARENA_DEFAULT_RESERVE;
    }

    // Align reserve size to OS page size
    size_t page_size = get_os_page_size();
    reserve_capacity = align_forward(reserve_capacity, page_size);

    // Reserve virtual address space without committing physical memory
    void* memory = mmap(NULL, reserve_capacity, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (memory == MAP_FAILED) {
        // Fallback to calloc if mmap fails
        memory = malloc(reserve_capacity);
        if (!memory) return NULL;
        Arena* arena = (Arena*)malloc(sizeof(Arena));
        arena->memory = (uint8_t*)memory;
        arena->capacity = reserve_capacity;
        arena->pos = 0;
        arena->commit_pos = reserve_capacity;
        arena->align = DEFAULT_ALIGNMENT;
        return arena;
    }

    Arena* arena = (Arena*)malloc(sizeof(Arena));
    arena->memory = (uint8_t*)memory;
    arena->capacity = reserve_capacity;
    arena->pos = 0;
    arena->commit_pos = 0;
    arena->align = DEFAULT_ALIGNMENT;

    return arena;
}

void arena_destroy(Arena* arena) {
    if (!arena) return;
    if (arena->memory) {
        munmap(arena->memory, arena->capacity);
    }
    free(arena);
}

void arena_clear(Arena* arena) {
    if (!arena) return;
    arena->pos = 0;
}

void* arena_push_align(Arena* arena, size_t size, size_t align) {
    if (!arena || size == 0) return NULL;

    size_t aligned_pos = align_forward(arena->pos, align);
    size_t new_pos = aligned_pos + size;

    if (new_pos > arena->capacity) {
        fprintf(stderr, "Fatal: Arena out of memory! Requested %zu, capacity %zu\n", new_pos, arena->capacity);
        return NULL;
    }

    // Commit pages if required
    if (new_pos > arena->commit_pos) {
        size_t page_size = get_os_page_size();
        size_t commit_chunk = ARENA_COMMIT_BLOCK_SIZE;
        if (commit_chunk < page_size) commit_chunk = page_size;

        size_t needed = new_pos - arena->commit_pos;
        size_t to_commit = align_forward(needed, commit_chunk);
        if (arena->commit_pos + to_commit > arena->capacity) {
            to_commit = arena->capacity - arena->commit_pos;
        }

        if (to_commit > 0) {
            int ret = mprotect(arena->memory + arena->commit_pos, to_commit, PROT_READ | PROT_WRITE);
            if (ret != 0) {
                fprintf(stderr, "Fatal: mprotect commit failed in arena_push\n");
                return NULL;
            }
            arena->commit_pos += to_commit;
        }
    }

    void* ptr = arena->memory + aligned_pos;
    arena->pos = new_pos;
    return ptr;
}

void* arena_push(Arena* arena, size_t size) {
    size_t align = arena ? arena->align : DEFAULT_ALIGNMENT;
    return arena_push_align(arena, size, align);
}

void* arena_push_zero(Arena* arena, size_t size) {
    void* ptr = arena_push(arena, size);
    if (ptr && size > 0) {
        memset(ptr, 0, size);
    }
    return ptr;
}

void* arena_push_copy(Arena* arena, const void* src, size_t size) {
    if (!src || size == 0) return NULL;
    void* ptr = arena_push(arena, size);
    if (ptr) {
        memcpy(ptr, src, size);
    }
    return ptr;
}

void arena_pop_to(Arena* arena, size_t pos) {
    if (!arena) return;
    if (pos < arena->pos) {
        arena->pos = pos;
    }
}

void arena_pop(Arena* arena, size_t size) {
    if (!arena) return;
    if (size >= arena->pos) {
        arena->pos = 0;
    } else {
        arena->pos -= size;
    }
}

Temp temp_begin(Arena* arena) {
    Temp temp;
    temp.arena = arena;
    temp.pos = arena ? arena->pos : 0;
    return temp;
}

void temp_end(Temp temp) {
    if (temp.arena) {
        arena_pop_to(temp.arena, temp.pos);
    }
}

// -----------------------------------------------------------------------------
// Scratch Arena Pool (Conflict-avoiding for nested functions)
// -----------------------------------------------------------------------------

#define SCRATCH_POOL_SIZE 4
static __thread Arena* tls_scratch_pool[SCRATCH_POOL_SIZE] = {0};

Temp scratch_begin(Arena** conflicts, size_t conflict_count) {
    // Lazy initialize thread-local scratch arenas
    for (size_t i = 0; i < SCRATCH_POOL_SIZE; i++) {
        if (!tls_scratch_pool[i]) {
            tls_scratch_pool[i] = arena_create(16 * 1024 * 1024); // 16 MB scratch arena
        }
    }

    // Select the first scratch arena not in conflicts
    Arena* selected = NULL;
    for (size_t i = 0; i < SCRATCH_POOL_SIZE; i++) {
        bool has_conflict = false;
        for (size_t c = 0; c < conflict_count; c++) {
            if (tls_scratch_pool[i] == conflicts[c]) {
                has_conflict = true;
                break;
            }
        }
        if (!has_conflict) {
            selected = tls_scratch_pool[i];
            break;
        }
    }

    if (!selected) {
        selected = tls_scratch_pool[0];
    }

    return temp_begin(selected);
}

void scratch_end(Temp scratch) {
    temp_end(scratch);
}

// -----------------------------------------------------------------------------
// String8 Implementation
// -----------------------------------------------------------------------------

String8 str8(const uint8_t* str, size_t size) {
    String8 s;
    s.str = (uint8_t*)str;
    s.size = size;
    return s;
}

String8 str8_from_cstr(const char* cstr) {
    if (!cstr) return str8_empty;
    String8 s;
    s.str = (uint8_t*)cstr;
    s.size = strlen(cstr);
    return s;
}

String8 str8_push_copy(Arena* arena, String8 s) {
    if (s.size == 0 || !s.str) return str8_empty;
    uint8_t* memory = (uint8_t*)arena_push(arena, s.size + 1);
    if (!memory) return str8_empty;
    memcpy(memory, s.str, s.size);
    memory[s.size] = '\0';
    return str8(memory, s.size);
}

String8 str8_push_cstr(Arena* arena, const char* cstr) {
    return str8_push_copy(arena, str8_from_cstr(cstr));
}

String8 str8_pushfv(Arena* arena, const char* fmt, va_list args) {
    if (!fmt) return str8_empty;
    va_list args_copy;
    va_copy(args_copy, args);
    int needed = vsnprintf(NULL, 0, fmt, args_copy);
    va_end(args_copy);

    if (needed < 0) return str8_empty;

    size_t size = (size_t)needed;
    uint8_t* buffer = (uint8_t*)arena_push(arena, size + 1);
    if (!buffer) return str8_empty;

    vsnprintf((char*)buffer, size + 1, fmt, args);
    buffer[size] = '\0';

    return str8(buffer, size);
}

String8 str8_pushf(Arena* arena, const char* fmt, ...) {
    va_list args;
    va_start(args, fmt);
    String8 result = str8_pushfv(arena, fmt, args);
    va_end(args);
    return result;
}

char* str8_to_cstr(Arena* arena, String8 s) {
    if (s.size == 0 || !s.str) {
        char* empty = (char*)arena_push(arena, 1);
        empty[0] = '\0';
        return empty;
    }
    char* cstr = (char*)arena_push(arena, s.size + 1);
    memcpy(cstr, s.str, s.size);
    cstr[s.size] = '\0';
    return cstr;
}

bool str8_equals(String8 a, String8 b) {
    if (a.size != b.size) return false;
    if (a.size == 0) return true;
    return memcmp(a.str, b.str, a.size) == 0;
}

bool str8_equals_ci(String8 a, String8 b) {
    if (a.size != b.size) return false;
    for (size_t i = 0; i < a.size; i++) {
        if (tolower(a.str[i]) != tolower(b.str[i])) return false;
    }
    return true;
}

String8 str8_substr(String8 s, size_t start, size_t length) {
    if (start >= s.size) return str8_empty;
    if (start + length > s.size) {
        length = s.size - start;
    }
    return str8(s.str + start, length);
}

String8 str8_concat(Arena* arena, String8 a, String8 b) {
    size_t total_size = a.size + b.size;
    if (total_size == 0) return str8_empty;

    uint8_t* memory = (uint8_t*)arena_push(arena, total_size + 1);
    if (a.size > 0 && a.str) memcpy(memory, a.str, a.size);
    if (b.size > 0 && b.str) memcpy(memory + a.size, b.str, b.size);
    memory[total_size] = '\0';

    return str8(memory, total_size);
}

String8 str8_trim(String8 s) {
    size_t start = 0;
    while (start < s.size && isspace(s.str[start])) {
        start++;
    }
    size_t end = s.size;
    while (end > start && isspace(s.str[end - 1])) {
        end--;
    }
    return str8_substr(s, start, end - start);
}

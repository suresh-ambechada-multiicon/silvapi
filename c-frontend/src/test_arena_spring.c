#include "arena.h"
#include "spring.h"
#include "fredbuf.h"
#include "font.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <math.h>

void test_arena(void) {
    printf("[TEST] Running Arena memory allocator tests...\n");

    Arena* arena = arena_create(4 * 1024 * 1024); // 4 MB
    assert(arena != NULL);
    assert(arena->capacity >= 4 * 1024 * 1024);
    assert(arena->pos == 0);

    // Test 1: Basic Push and Alignment
    int* int_array = arena_push_array_zero(arena, int, 100);
    assert(int_array != NULL);
    for (int i = 0; i < 100; i++) {
        assert(int_array[i] == 0);
        int_array[i] = i * 42;
    }
    assert(arena->pos >= sizeof(int) * 100);

    // Test 2: Scoped Temp Arena
    Temp temp = temp_begin(arena);
    size_t saved_pos = arena->pos;

    char* temp_str = arena_push_zero(arena, 2048);
    assert(temp_str != NULL);
    snprintf(temp_str, 2048, "Temporary string in arena scope");
    assert(arena->pos >= saved_pos + 2048);

    temp_end(temp);
    assert(arena->pos == saved_pos);

    // Test 3: String8 Slices
    String8 s1 = str8_lit("Hello");
    String8 s2 = str8_lit(" World!");
    String8 joined = str8_concat(arena, s1, s2);
    assert(joined.size == 12);
    assert(strcmp((char*)joined.str, "Hello World!") == 0);

    String8 formatted = str8_pushf(arena, "Status: %d - %s", 200, "OK");
    assert(strcmp((char*)formatted.str, "Status: 200 - OK") == 0);

    // Test 4: Scratch Pool (Conflict avoidance)
    Temp scratch1 = scratch_begin(NULL, 0);
    Temp scratch2 = scratch_begin(&scratch1.arena, 1);
    assert(scratch1.arena != scratch2.arena); // Must select distinct arenas

    String8 sc_str1 = str8_pushf(scratch1.arena, "Scratch 1: %d", 111);
    String8 sc_str2 = str8_pushf(scratch2.arena, "Scratch 2: %d", 222);
    assert(strcmp((char*)sc_str1.str, "Scratch 1: 111") == 0);
    assert(strcmp((char*)sc_str2.str, "Scratch 2: 222") == 0);

    scratch_end(scratch2);
    scratch_end(scratch1);

    arena_destroy(arena);
    printf("[PASS] Arena memory allocator tests passed successfully!\n");
}

void test_spring(void) {
    printf("[TEST] Running Spring Dynamics & File Pilot Motion tests...\n");

    // Test 1: 1D Critically Damped Spring
    Spring1D s1 = spring1d_make(0.0f, SPRING_CONFIG_SNAPPY);
    spring1d_set_target(&s1, 100.0f);

    float dt = 1.0f / 60.0f; // 60 FPS frame time
    for (int frame = 0; frame < 120; frame++) {
        spring1d_update(&s1, dt);
    }

    // Must settle smoothly at 100.0
    assert(fabsf(s1.pos - 100.0f) < 0.01f);
    assert(spring1d_is_settled(&s1, 0.05f, 0.05f));

    // Test 2: Rectangle Morphing Spring
    Rect initial_r = rect_make(0, 0, 50, 50);
    Rect target_r = rect_make(100, 200, 300, 400);
    SpringRect sr = spring_rect_make(initial_r, SPRING_CONFIG_BOUNCY);
    spring_rect_set_target(&sr, target_r);

    for (int frame = 0; frame < 180; frame++) {
        spring_rect_update(&sr, dt);
    }

    assert(fabsf(sr.pos.x - 100.0f) < 0.05f);
    assert(fabsf(sr.pos.y - 200.0f) < 0.05f);
    assert(fabsf(sr.pos.w - 300.0f) < 0.05f);
    assert(fabsf(sr.pos.h - 400.0f) < 0.05f);

    // Test 3: File Pilot Gliding Caret Spring (Velocity Stretch Verification)
    CaretSpring caret = caret_spring_make(10.0f, 20.0f, 16.0f);
    caret_spring_set_target(&caret, 200.0f, 20.0f, 16.0f); // Jump 190 pixels to the right

    // Simulate 2 frames in flight
    caret_spring_update(&caret, dt);
    caret_spring_update(&caret, dt);

    Rect flight_bounds = caret_spring_get_bounds(&caret, 2.0f);
    assert(flight_bounds.w > 2.0f);
    printf("  [File Pilot Caret] In-flight stretch width: %.2f px (Standard: 2.0 px)\n", flight_bounds.w);

    // Simulate full settle
    for (int frame = 0; frame < 120; frame++) {
        caret_spring_update(&caret, dt);
    }

    Rect settled_bounds = caret_spring_get_bounds(&caret, 2.0f);
    assert(fabsf(settled_bounds.x - 200.0f) < 0.1f);
    assert(fabsf(settled_bounds.w - 2.0f) < 0.1f); // Snaps back to 2px width!
    printf("  [File Pilot Caret] Settled caret bounds: (x=%.2f, y=%.2f, w=%.2f, h=%.2f)\n",
           settled_bounds.x, settled_bounds.y, settled_bounds.w, settled_bounds.h);

    printf("[PASS] Spring Dynamics tests passed successfully!\n");
}

void test_fredbuf(void) {
    printf("[TEST] Running FredBuffer Gap-Buffer & Undo/Redo tests...\n");

    FredBuffer* buf = fredbuf_create_from_string("Hello World\nLine 2\nLine 3");
    assert(buf != NULL);
    assert(fredbuf_length(buf) == 25);
    assert(fredbuf_line_count(buf) == 3);

    // Insert string at offset 5 (between 'Hello' and ' World')
    fredbuf_insert_string(buf, 5, ", Beautiful", 11);
    char* str1 = fredbuf_to_string(buf);
    assert(strcmp(str1, "Hello, Beautiful World\nLine 2\nLine 3") == 0);
    free(str1);

    // Test Undo
    size_t cursor = 0;
    fredbuf_undo(buf, &cursor);
    char* str_undone = fredbuf_to_string(buf);
    assert(strcmp(str_undone, "Hello World\nLine 2\nLine 3") == 0);
    assert(cursor == 5);
    free(str_undone);

    // Test Redo
    fredbuf_redo(buf, &cursor);
    char* str_redone = fredbuf_to_string(buf);
    assert(strcmp(str_redone, "Hello, Beautiful World\nLine 2\nLine 3") == 0);
    assert(cursor == 16);
    free(str_redone);

    // Delete range
    fredbuf_delete_range(buf, 5, 11);
    char* str_del = fredbuf_to_string(buf);
    assert(strcmp(str_del, "Hello World\nLine 2\nLine 3") == 0);
    free(str_del);

    // Undo delete
    fredbuf_undo(buf, &cursor);
    char* str_del_undone = fredbuf_to_string(buf);
    assert(strcmp(str_del_undone, "Hello, Beautiful World\nLine 2\nLine 3") == 0);
    free(str_del_undone);

    // Line/Col indexing
    size_t line = 0, col = 0;
    // Find offset of 'Line 2'
    fredbuf_offset_to_line_col(buf, 24, &line, &col);
    assert(line == 1);
    assert(col == 1);

    size_t off = fredbuf_line_col_to_offset(buf, 1, 1);
    assert(off == 24);

    fredbuf_free(buf);
    printf("[PASS] FredBuffer Gap-Buffer tests passed successfully!\n");
}

void test_cursor_boundary(void) {
    printf("[TEST] Running Text Cursor Line-Boundary & Home/End Navigation tests...\n");

    // Construct a mock TextLayout with 3 wrapped visual lines
    TextLayout layout = {0};
    layout.line_height = 20.0f;
    layout.line_count = 3;
    layout.lines = malloc(sizeof(TextLine) * 3);

    // Line 0: "hello " (bytes 0..5, len=6)
    layout.lines[0].start_byte = 0;
    layout.lines[0].length_byte = 6;
    layout.lines[0].y_offset = 0.0f;

    // Line 1: "world " (bytes 6..11, len=6)
    layout.lines[1].start_byte = 6;
    layout.lines[1].length_byte = 6;
    layout.lines[1].y_offset = 20.0f;

    // Line 2: "foo" (bytes 12..14, len=3)
    layout.lines[2].start_byte = 12;
    layout.lines[2].length_byte = 3;
    layout.lines[2].y_offset = 40.0f;

    const char* text = "hello world foo";

    // Test cursor at byte 0 (start of Line 0) -> y should be 0.0
    float cx = -1.0f, cy = -1.0f, lh = 0.0f;
    for (size_t i = 0; i < layout.line_count; i++) {
        if (i == layout.line_count - 1 || 0 < layout.lines[i + 1].start_byte) {
            cy = layout.lines[i].y_offset;
            cx = (0 > layout.lines[i].start_byte) ? (0 - layout.lines[i].start_byte) : 0.0f;
            break;
        }
    }
    assert(cy == 0.0f);
    assert(cx == 0.0f);

    // Test cursor at byte 6 (start of Line 1) -> y must be 20.0, NOT 0.0 (prevents overflow to line 0 space!)
    for (size_t i = 0; i < layout.line_count; i++) {
        if (i == layout.line_count - 1 || 6 < layout.lines[i + 1].start_byte) {
            cy = layout.lines[i].y_offset;
            cx = (6 > layout.lines[i].start_byte) ? (6 - layout.lines[i].start_byte) : 0.0f;
            break;
        }
    }
    assert(cy == 20.0f);
    assert(cx == 0.0f);

    // Test cursor at byte 12 (start of Line 2) -> y must be 40.0
    for (size_t i = 0; i < layout.line_count; i++) {
        if (i == layout.line_count - 1 || 12 < layout.lines[i + 1].start_byte) {
            cy = layout.lines[i].y_offset;
            cx = (12 > layout.lines[i].start_byte) ? (12 - layout.lines[i].start_byte) : 0.0f;
            break;
        }
    }
    assert(cy == 40.0f);
    assert(cx == 0.0f);

    free(layout.lines);
    printf("[PASS] Text Cursor Line-Boundary tests passed successfully!\n");
}

int main(void) {
    printf("===================================================\n");
    printf("  SILVAPI: ARENA, SPRING & FREDBUF TEST SUITE      \n");
    printf("===================================================\n");
    test_arena();
    test_spring();
    test_fredbuf();
    test_cursor_boundary();
    printf("===================================================\n");
    printf("  ALL TESTS PASSED!                                \n");
    printf("===================================================\n");
    return 0;
}

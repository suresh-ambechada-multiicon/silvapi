#ifndef UI_COMPONENTS_CODE_EDITOR_H
#define UI_COMPONENTS_CODE_EDITOR_H

#include "ui/layout.h"
#include "ui/style.h"
#include "ui/editbuf.h"

// ---------------------------------------------------------------------------
// Code editor / viewer, over a FredBuffer.
//
// Syntax colouring comes from a caller-supplied tokenizer, so the component
// knows nothing about JSON, XML, or anything else. The app owns its languages;
// this owns caret, selection, scrolling, and drawing.
//
// Text lives in a FredBuffer rather than a flat char array. That is not an
// implementation detail: the gap buffer already maintains a cached line index
// with a dirty flag, converts offset<->line/col, inserts and deletes at the gap
// without moving the whole document, searches with Boyer-Moore, and keeps undo
// stacks. Reimplementing any of that on a char* means an O(document) memmove per
// keystroke and a full rescan per frame to find line boundaries.
//
// Lines are virtualized: only the visible span is drawn, and the whole viewport
// is TWO layout elements (gutter + content) rather than one per line. A 40k-line
// response body therefore costs the same per frame as a 40-line one -- turning
// each line into layout elements would put tens of thousands of boxes through
// the solver for text that is all the same height anyway.
//
// Soft wrap is supported and makes a visual row differ from a logical line, so
// every caret, click, and scroll calculation goes through a mapping.
//
// The mapping is arithmetic rather than measured: the font is monospaced, so a
// row holds floor(content_width / advance) characters and a line of L characters
// occupies ceil(L / that) rows. That keeps the row COUNT O(1) per line, which is
// what lets a 14k-line document know its own scroll height without measuring
// 14k lines. A prefix sum over those counts maps visual row to logical line.
//
// The cost of that choice: breaks land mid-token rather than at word boundaries.
// Word-aware wrapping needs real break positions per line, which gives up the
// O(1) row count and needs a per-line cache invalidated on every width change.
// ---------------------------------------------------------------------------

typedef struct {
    uint32_t start;   // byte offset within the line
    uint32_t length;
    uint8_t  kind;    // index into the caller's colour palette
} UiCodeToken;

// Tokenizes ONE line. Returns how many tokens were written. Bytes not covered by
// any token are drawn in the default foreground, so a tokenizer may emit only
// the spans it cares about.
typedef size_t (*UiCodeTokenizeFn)(const char* line, size_t len,
                                   UiCodeToken* out, size_t max, void* user);

// The props struct itself may be a temporary (a compound literal is fine), but
// every pointer INSIDE it -- buffer, palette, search, tokenize_user -- must stay
// valid until the end of the frame, because drawing happens after the layout
// solver runs, not during this call.
typedef struct {
    const char* id;
    const char* class;

    // Caller-owned. The component mutates it through the fredbuf API when not
    // readonly, so undo/redo history accumulates in the buffer itself.
    FredBuffer* buffer;
    bool        readonly;

    bool  show_line_numbers;
    bool  soft_wrap;      // wrap long lines instead of scrolling horizontally
    float line_height;    // 0 = the mono font's natural line height
    float max_height;     // 0 = fill the parent
    int   tab_width;      // 0 = 4

    UiCodeTokenizeFn tokenize;
    void*            tokenize_user;
    const Color*     palette;        // indexed by UiCodeToken.kind
    size_t           palette_size;

    // Highlights every occurrence; does not move the caret.
    const char* search;
} UiCodeEditorProps;

typedef struct {
    bool   changed;
    size_t line_count;
    size_t caret_line;      // 0-based
    size_t caret_col;       // 0-based, in bytes
    size_t caret_offset;
    size_t first_visible_line;   // logical line at the top of the viewport
    size_t visible_rows;         // visual rows drawn; the virtualization receipt
    size_t total_rows;           // visual rows in the document
    size_t search_hits;
} UiCodeEditorResult;

UiCodeEditorResult ui_code_editor(const UiCodeEditorProps* props);

#endif // UI_COMPONENTS_CODE_EDITOR_H

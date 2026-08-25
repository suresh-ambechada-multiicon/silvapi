#ifndef UI_FUZZY_H
#define UI_FUZZY_H

#include "base/base.h"

// ---------------------------------------------------------------------------
// Subsequence matching with ranking.
//
// Its own unit because three places need the same behaviour -- command palette,
// sidebar search, table filter -- and a search that ranks differently in each
// place feels broken even when every implementation is individually reasonable.
//
// Case-insensitive. Contiguous runs and start-of-word hits score higher, so
// "tef" ranks "Toggle Expand Folder" above a string that merely contains those
// letters scattered. Returns -1 when the needle is not a subsequence.
// ---------------------------------------------------------------------------

int fuzzy_score(const char* haystack, const char* needle);

// True when `needle` is a subsequence of `haystack`, ignoring rank.
bool fuzzy_matches(const char* haystack, const char* needle);

#endif // UI_FUZZY_H

#include "ui/fuzzy.h"
#include <ctype.h>

int fuzzy_score(const char* haystack, const char* needle) {
    // An empty needle matches everything with a neutral score, which is what
    // makes "no filter typed yet" fall out of the same code path.
    if (!needle || !*needle) return 0;
    if (!haystack || !*haystack) return -1;

    int score = 0;
    int run = 0;
    bool at_boundary = true;

    const char* h = haystack;
    const char* n = needle;

    while (*h && *n) {
        if (tolower((unsigned char)*h) == tolower((unsigned char)*n)) {
            score += 1 + run;                 // contiguity compounds
            if (at_boundary) score += 4;      // start-of-word is a strong signal
            run++;
            n++;
        } else {
            run = 0;
        }
        at_boundary = (*h == ' ' || *h == '-' || *h == '_' ||
                       *h == '/' || *h == '.' || *h == ':');
        h++;
    }

    if (*n) return -1;   // needle not fully consumed

    // Shorter haystacks win ties: an exact short label should beat a long one
    // that happens to contain the same letters.
    score -= (int)(strlen(haystack) / 8);
    return score;
}

bool fuzzy_matches(const char* haystack, const char* needle) {
    return fuzzy_score(haystack, needle) >= 0;
}

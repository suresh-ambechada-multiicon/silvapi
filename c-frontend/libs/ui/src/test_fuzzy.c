#include "ui/fuzzy.h"

static int g_fail = 0;

static void check(bool cond, const char* what) {
    if (!cond) { fprintf(stderr, "FAIL: %s\n", what); g_fail++; }
}

static void check_better(const char* a, const char* b, const char* needle) {
    int sa = fuzzy_score(a, needle), sb = fuzzy_score(b, needle);
    if (sa <= sb) {
        fprintf(stderr, "FAIL: \"%s\"(%d) should outrank \"%s\"(%d) for \"%s\"\n",
                a, sa, b, sb, needle);
        g_fail++;
    }
}

int main(void) {
    // Empty needle matches anything, so "nothing typed yet" needs no special case.
    check(fuzzy_score("anything", "") == 0, "empty needle is neutral");
    check(fuzzy_score("", "x") < 0, "empty haystack cannot match");
    check(fuzzy_score(NULL, "x") < 0, "NULL haystack is safe");

    // Subsequence, not substring.
    check(fuzzy_matches("Toggle Expand Folder", "tef"), "initials match");
    check(fuzzy_matches("Toggle Expand Folder", "expand"), "word match");
    check(!fuzzy_matches("Toggle Expand Folder", "xyz"), "non-subsequence rejected");
    check(!fuzzy_matches("abc", "abcd"), "needle longer than haystack rejected");

    // Case insensitivity both ways.
    check(fuzzy_matches("Open New Tab", "ONT"), "uppercase needle");
    check(fuzzy_matches("OPEN NEW TAB", "ont"), "uppercase haystack");

    // Ranking: word-boundary initials beat scattered letters.
    check_better("Toggle Expand Folder", "The lateral effects", "tef");

    // Ranking: contiguous run beats a split one.
    check_better("Split Right", "Set panel is right", "split");

    // Ranking: shorter wins ties.
    check_better("Options", "Options and other preferences panel", "options");

    // Boundary characters other than space count as word starts.
    check(fuzzy_matches("open-new-tab", "ont"), "hyphen is a boundary");
    check(fuzzy_matches("some/path/here", "sph"), "slash is a boundary");
    check_better("open-new-tab", "opponent nut ab", "ont");

    if (g_fail == 0) { printf("fuzzy: all checks passed\n"); return 0; }
    printf("fuzzy: %d FAILURES\n", g_fail);
    return 1;
}

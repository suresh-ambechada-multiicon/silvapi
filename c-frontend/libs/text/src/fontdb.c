#include "text_internal.h"
#include <fontconfig/fontconfig.h>

// ---------------------------------------------------------------------------
// Font discovery through fontconfig.
//
// Replaces the previous hardcoded candidate lists (/usr/share/fonts/Adwaita/...,
// /usr/share/fonts/TTF/...), which silently produced no font at all on any
// distro that lays its fonts out differently.
// ---------------------------------------------------------------------------

static bool s_fc_ready = false;

static bool ensure_fc(void) {
    if (s_fc_ready) return true;
    s_fc_ready = FcInit() == FcTrue;
    if (!s_fc_ready) fprintf(stderr, "[Text] fontconfig init failed\n");
    return s_fc_ready;
}

static char* fc_best_match(FcPattern* pat) {
    FcConfigSubstitute(NULL, pat, FcMatchPattern);
    FcDefaultSubstitute(pat);

    FcResult result = FcResultNoMatch;
    FcPattern* match = FcFontMatch(NULL, pat, &result);
    if (!match) return NULL;

    char* out = NULL;
    FcChar8* file = NULL;
    if (FcPatternGetString(match, FC_FILE, 0, &file) == FcResultMatch && file) {
        out = strdup((const char*)file);
    }
    FcPatternDestroy(match);
    return out;
}

char* fontdb_find(const char* family, bool bold, bool mono) {
    if (!ensure_fc()) return NULL;

    FcPattern* pat = FcPatternCreate();
    if (!pat) return NULL;

    if (family && *family) {
        FcPatternAddString(pat, FC_FAMILY, (const FcChar8*)family);
    } else if (mono) {
        FcPatternAddString(pat, FC_FAMILY, (const FcChar8*)"monospace");
    } else {
        FcPatternAddString(pat, FC_FAMILY, (const FcChar8*)"sans-serif");
    }

    FcPatternAddInteger(pat, FC_WEIGHT, bold ? FC_WEIGHT_BOLD : FC_WEIGHT_REGULAR);
    FcPatternAddInteger(pat, FC_SLANT, FC_SLANT_ROMAN);
    if (mono) FcPatternAddInteger(pat, FC_SPACING, FC_MONO);
    // Only scalable faces: the engine rasterizes at arbitrary pixel sizes.
    FcPatternAddBool(pat, FC_SCALABLE, FcTrue);

    char* path = fc_best_match(pat);
    FcPatternDestroy(pat);
    return path;
}

size_t fontdb_fallback_chain(char** out_paths, size_t max) {
    if (!ensure_fc() || !out_paths || max == 0) return 0;

    // Families chosen for coverage breadth rather than looks: between them they
    // cover Latin/Greek/Cyrillic, CJK, emoji, and a broad symbol range. Missing
    // families simply resolve to fontconfig's own substitute.
    static const char* families[] = {
        "Noto Sans", "DejaVu Sans", "Noto Color Emoji", "Noto Sans Symbols 2",
        "Noto Sans CJK SC", "Noto Sans Arabic", "Noto Sans Devanagari",
    };

    size_t found = 0;
    for (size_t i = 0; i < sizeof(families) / sizeof(families[0]) && found < max; i++) {
        FcPattern* pat = FcPatternCreate();
        if (!pat) continue;
        FcPatternAddString(pat, FC_FAMILY, (const FcChar8*)families[i]);
        FcPatternAddBool(pat, FC_SCALABLE, FcTrue);

        // Require the family actually be present, otherwise fontconfig happily
        // returns the default sans for every entry and the chain is all dupes.
        FcConfigSubstitute(NULL, pat, FcMatchPattern);
        FcDefaultSubstitute(pat);
        FcResult res = FcResultNoMatch;
        FcPattern* match = FcFontMatch(NULL, pat, &res);
        if (match) {
            FcChar8* fam = NULL;
            FcChar8* file = NULL;
            if (FcPatternGetString(match, FC_FAMILY, 0, &fam) == FcResultMatch &&
                FcPatternGetString(match, FC_FILE, 0, &file) == FcResultMatch &&
                fam && file && strcasecmp((const char*)fam, families[i]) == 0) {

                bool dup = false;
                for (size_t k = 0; k < found; k++) {
                    if (strcmp(out_paths[k], (const char*)file) == 0) { dup = true; break; }
                }
                if (!dup) out_paths[found++] = strdup((const char*)file);
            }
            FcPatternDestroy(match);
        }
        FcPatternDestroy(pat);
    }
    return found;
}

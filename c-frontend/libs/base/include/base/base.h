#ifndef BASE_BASE_H
#define BASE_BASE_H

// Common libc surface shared by every layer, plus core geometry types.
// Kept deliberately small: no threading, no platform headers, no app types.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>
#include <math.h>
#include <time.h>

#include "base/geom.h"

#endif // BASE_BASE_H

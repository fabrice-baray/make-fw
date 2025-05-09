#include <assert.h>
#include "libB.h"

#ifdef LIBB
int fb() { assert(1); return 1; }
#endif


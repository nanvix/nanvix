/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/times.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can retrieve process times with `times()`.
void test_times(void)
{
    fprintf(stderr, "testing times() ... ");

    struct tms buf = {0};

    // Call `times()` and check the result.
    memset(&buf, 0, sizeof(buf));
    assert(times(&buf) != (clock_t)-1);

    // Validate the values returned in the `tms` structure.
    // TODO: Investigate why `tms` is filled with zeroes and uncomment the assertions.
#if 0
    assert(buf.tms_utime > 0);
    assert(buf.tms_stime > 0);
    assert(buf.tms_cutime > 0);
    assert(buf.tms_cstime > 0);
#endif

    // Call `times()` and check the result.
    assert(times(NULL) != (clock_t)-1);

    fprintf(stderr, "passed\n");
}

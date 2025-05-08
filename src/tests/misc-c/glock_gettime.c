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
#include <time.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can get the current time of a clock with `clock_gettime()`.
void test_clock_gettime(void)
{
    fprintf(stderr, "testing clock_gettime() ... ");

    struct timespec ts = {0};

    // Get the current time of the monotonic clock and check the result.
    memset(&ts, 0, sizeof(ts));
    assert(clock_gettime(CLOCK_MONOTONIC, &ts) == 0);
    assert(ts.tv_sec >= 0);
    assert(ts.tv_nsec >= 0);

    // Get the current time of the real-time clock and check the result.
    memset(&ts, 0, sizeof(ts));
    assert(clock_gettime(CLOCK_REALTIME, &ts) == 0);
    assert(ts.tv_sec >= 0);
    assert(ts.tv_nsec >= 0);

    // Get the current time of the process CPU-time clock and check the result.
    memset(&ts, 0, sizeof(ts));
    assert(clock_gettime(CLOCK_PROCESS_CPUTIME_ID, &ts) == -1);
    assert(errno == ENOTSUP);
    errno = 0;

    // Get the current time of the thread CPU-time clock and check the result.
    memset(&ts, 0, sizeof(ts));
    assert(clock_gettime(CLOCK_THREAD_CPUTIME_ID, &ts) == -1);
    assert(errno == ENOTSUP);
    errno = 0;

    fprintf(stderr, "passed\n");
}

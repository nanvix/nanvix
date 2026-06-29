/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <stdio.h>
#include <time.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether `sleep()` suspends the caller for the requested number of seconds.
void test_sleep(void)
{
    fprintf(stderr, "testing sleep() ... ");

    // A zero-second request returns immediately with nothing left unslept.
    assert(sleep(0) == 0);

    // A one-second request sleeps the full interval and reports no unslept time.
    struct timespec before = {0, 0};
    struct timespec after = {0, 0};
    assert(clock_gettime(CLOCK_MONOTONIC, &before) == 0);
    assert(sleep(1) == 0);
    assert(clock_gettime(CLOCK_MONOTONIC, &after) == 0);

    // The caller must have been suspended for about one second.
    long long elapsed_ns = ((long long)after.tv_sec - (long long)before.tv_sec) * 1000000000LL +
                           ((long long)after.tv_nsec - (long long)before.tv_nsec);
    assert(elapsed_ns >= 900000000LL);

    fprintf(stderr, "passed\n");
}

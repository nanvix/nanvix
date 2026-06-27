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

// Tests whether we can get the resolution of a clock with `clock_getres()`.
void test_clock_getres(void)
{
    fprintf(stderr, "testing clock_getres() ... ");

    // Assert size of `timespec` structure.
    STATIC_ASSERT_SIZE(struct timespec, sizeof(time_t) + sizeof(long));

    struct timespec res = {0};

    // Get the resolution of the monotonic clock and check the result.
    memset(&res, 0, sizeof(res));
    assert(clock_getres(CLOCK_MONOTONIC, &res) == 0);
    assert(res.tv_sec >= 0);
    assert(res.tv_nsec >= 0);

    // Get the resolution of the real-time clock and check the result.
    memset(&res, 0, sizeof(res));
    assert(clock_getres(CLOCK_REALTIME, &res) == 0);
    assert(res.tv_sec >= 0);
    assert(res.tv_nsec >= 0);

    // Get the resolution of the process CPU-time clock and check the result.
    memset(&res, 0, sizeof(res));
    assert(clock_getres(CLOCK_PROCESS_CPUTIME_ID, &res) == -1);
    assert(errno == ENOTSUP);
    errno = 0;

    // Get the resolution of the thread CPU-time clock and check the result.
    memset(&res, 0, sizeof(res));
    assert(clock_getres(CLOCK_THREAD_CPUTIME_ID, &res) == -1);
    assert(errno == ENOTSUP);
    errno = 0;

    fprintf(stderr, "passed\n");
}

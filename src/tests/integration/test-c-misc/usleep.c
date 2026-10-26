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
// Constants
//==================================================================================================

// Number of nanoseconds in a microsecond.
#define NANOSECONDS_PER_MICROSECOND 1000LL

// Number of nanoseconds in a second.
#define NANOSECONDS_PER_SECOND 1000000000LL

// Sub-second sleep interval, in microseconds.
#define SHORT_SLEEP_USEC 250000U

// Sleep interval of more than one second, in microseconds. It has a sub-second part large enough
// that a sleep that skips either part falls short of the tolerance allowed by `check_usleep()`.
#define LONG_SLEEP_USEC 1250000U

//==================================================================================================
// Private Functions
//==================================================================================================

// Sleeps for `usec` microseconds with `usleep()` and checks that the caller was suspended for about
// the requested interval.
static void check_usleep(unsigned int usec)
{
    struct timespec before = {0, 0};
    struct timespec after = {0, 0};
    assert(clock_gettime(CLOCK_MONOTONIC, &before) == 0);
    assert(usleep(usec) == 0);
    assert(clock_gettime(CLOCK_MONOTONIC, &after) == 0);

    // The caller must have been suspended for at least 90% of the requested interval.
    long long requested_ns = (long long)usec * NANOSECONDS_PER_MICROSECOND;
    long long elapsed_ns =
        ((long long)after.tv_sec - (long long)before.tv_sec) * NANOSECONDS_PER_SECOND +
        ((long long)after.tv_nsec - (long long)before.tv_nsec);
    assert(elapsed_ns >= requested_ns - requested_ns / 10);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether `usleep()` suspends the caller for the requested number of microseconds.
void test_usleep(void)
{
    fprintf(stderr, "testing usleep() ... ");

    // A zero-microsecond request has no effect.
    assert(usleep(0) == 0);

    // A sub-second request sleeps for the requested interval.
    check_usleep(SHORT_SLEEP_USEC);

    // A request of one second or more is honored rather than rejected, and both its whole-second
    // and sub-second parts are slept.
    check_usleep(LONG_SLEEP_USEC);

    fprintf(stderr, "passed\n");
}

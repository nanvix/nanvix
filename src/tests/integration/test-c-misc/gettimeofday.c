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
#include <sys/time.h>
#include <time.h>

//==================================================================================================
// Constants
//==================================================================================================

// Number of microseconds in a second.
#define MICROSECONDS_PER_SECOND 1000000LL

// Number of nanoseconds in a microsecond.
#define NANOSECONDS_PER_MICROSECOND 1000LL

// Value that output fields hold before each call, so that fields left unset by the call are caught.
#define POISON (-1)

//==================================================================================================
// Private Functions
//==================================================================================================

// Converts a time read with `clock_gettime()` to microseconds, truncating the remainder.
static long long timespec_to_usec(const struct timespec *ts)
{
    return (long long)ts->tv_sec * MICROSECONDS_PER_SECOND +
           (long long)ts->tv_nsec / NANOSECONDS_PER_MICROSECOND;
}

// Converts a time read with `gettimeofday()` to microseconds.
static long long timeval_to_usec(const struct timeval *tv)
{
    return (long long)tv->tv_sec * MICROSECONDS_PER_SECOND + (long long)tv->tv_usec;
}

// Gets the time of day with `gettimeofday()`, passing `tz` as the time zone argument, and checks
// that it is the time of `CLOCK_REALTIME` at microsecond resolution.
static void check_time_of_day(struct timezone *tz)
{
    struct timespec before = {0, 0};
    struct timespec after = {0, 0};
    struct timeval tv = {POISON, POISON};

    assert(clock_gettime(CLOCK_REALTIME, &before) == 0);
    assert(gettimeofday(&tv, tz) == 0);
    assert(clock_gettime(CLOCK_REALTIME, &after) == 0);

    // The microseconds must be below one second.
    assert(tv.tv_usec >= 0);
    assert(tv.tv_usec < MICROSECONDS_PER_SECOND);

    // The time must fall between the readings of `CLOCK_REALTIME` taken right before and right
    // after it, once they are truncated to microseconds as well.
    assert(timespec_to_usec(&before) <= timeval_to_usec(&tv));
    assert(timeval_to_usec(&tv) <= timespec_to_usec(&after));
}

// Checks that `tz` holds the time zone of the system clock, which is UTC.
static void check_time_zone(const struct timezone *tz)
{
    assert(tz->tz_minuteswest == 0);
    assert(tz->tz_dsttime == 0);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can get the time of day with `gettimeofday()`.
void test_gettimeofday(void)
{
    fprintf(stderr, "testing gettimeofday() ... ");

    // Get the time only.
    check_time_of_day(NULL);

    // Get both the time and the time zone.
    struct timezone tz = {POISON, POISON};
    check_time_of_day(&tz);
    check_time_zone(&tz);

    // Get the time zone only.
    tz.tz_minuteswest = POISON;
    tz.tz_dsttime = POISON;
    assert(gettimeofday(NULL, &tz) == 0);
    check_time_zone(&tz);

    // Get neither the time nor the time zone.
    assert(gettimeofday(NULL, NULL) == 0);

    fprintf(stderr, "passed\n");
}

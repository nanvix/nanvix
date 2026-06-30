/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

// <unistd.h> is included first and on its own so the feature-test-macro checks below attribute the
// macros specifically to <unistd.h>, not to any later header.
#include <unistd.h>

//==================================================================================================
// Compile-Time Checks
//==================================================================================================

// <unistd.h> must advertise the POSIX timers and monotonic-clock options. Portable software (e.g.
// libc++'s chrono.cpp) gates the clock_gettime(CLOCK_MONOTONIC) path on `_POSIX_TIMERS > 0`, so a
// missing or non-positive macro is a hard regression.
#if !defined(_POSIX_TIMERS) || (_POSIX_TIMERS <= 0)
#error "_POSIX_TIMERS is not advertised by <unistd.h>"
#endif

#if !defined(_POSIX_MONOTONIC_CLOCK) || (_POSIX_MONOTONIC_CLOCK <= 0)
#error "_POSIX_MONOTONIC_CLOCK is not advertised by <unistd.h>"
#endif

//==================================================================================================
// Remaining Imports
//==================================================================================================

#include <assert.h>
#include <errno.h>
#include <stddef.h>
#include <string.h>
#include <time.h>

//==================================================================================================
// Private Functions
//==================================================================================================

// Confirms, at run time, that the feature-test macros are positive (mirroring the compile-time
// guard above so the check is also exercised at run time).
static void test_macros_advertised(void)
{
    assert(_POSIX_TIMERS > 0);
    assert(_POSIX_MONOTONIC_CLOCK > 0);
}

// Confirms the advertisement is truthful: CLOCK_MONOTONIC is actually serviced by clock_gettime()
// and clock_getres(), and successive reads are non-decreasing.
static void test_monotonic_clock_works(void)
{
    struct timespec res = {0};
    struct timespec first = {0};
    struct timespec second = {0};

    assert(clock_getres(CLOCK_MONOTONIC, &res) == 0);
    assert(res.tv_sec >= 0);
    assert(res.tv_nsec >= 0);

    assert(clock_gettime(CLOCK_MONOTONIC, &first) == 0);
    assert(first.tv_sec >= 0);
    assert(first.tv_nsec >= 0);

    assert(clock_gettime(CLOCK_MONOTONIC, &second) == 0);

    // The monotonic clock must never run backwards.
    assert(second.tv_sec > first.tv_sec ||
           (second.tv_sec == first.tv_sec && second.tv_nsec >= first.tv_nsec));
}

// Confirms nanosleep(), part of the advertised POSIX timers subset, suspends and returns success.
static void test_nanosleep_works(void)
{
#ifndef __hyperlight__
    struct timespec request = {0};
    request.tv_sec = 0;
    request.tv_nsec = 1000000; // 1 ms.
    assert(nanosleep(&request, NULL) == 0);
#endif
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests that <unistd.h> advertises the POSIX timers options and that the advertised
 * monotonic-clock functionality is implemented.
 *
 * @param argc Number of command-line arguments.
 * @param argv List of command-line arguments.
 *
 * @returns Always returns zero. If a test fails, the program will abort.
 */
int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    // Assert command-line arguments.
    assert(argc == 1);
    assert(argv[0] != NULL);
    assert(argv[1] == NULL);
    assert(strcmp(argv[0], "test-c-posix-timers.elf") == 0);

    test_macros_advertised();
    test_monotonic_clock_works();
    test_nanosleep_works();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}

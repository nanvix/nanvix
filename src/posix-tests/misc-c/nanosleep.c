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

// Tests whether we can sleep for a given amount of time with `nanosleep()`.
void test_nanosleep(void)
{
    fprintf(stderr, "testing nanosleep() ... ");

    struct timespec req = {
        .tv_sec = 1,
        .tv_nsec = 0,
    };

    assert(nanosleep(&req, NULL) == 0);

    fprintf(stderr, "passed\n");
}

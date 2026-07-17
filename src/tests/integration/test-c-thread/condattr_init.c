/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <pthread.h>
#include <stdio.h>
#include <time.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests if condition variable attributes can be initialized.
void test_pthread_condattr_init(void)
{
    fprintf(stderr, "testing pthread_condattr_init() ... ");

    pthread_condattr_t attr = {
        .is_initialized = 0,
        .clock = CLOCK_MONOTONIC,
    };
    int ret = pthread_condattr_init(&attr);
    assert(ret == 0);

    // Initialization must overwrite caller-provided storage with default values.
    assert(attr.is_initialized != 0);
    assert(attr.clock == CLOCK_REALTIME);

    // Invalid pointers must be rejected.
    ret = pthread_condattr_init(NULL);
    assert(ret != 0);

    fprintf(stderr, "passed\n");
}

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

// Tests if the clock attribute can be retrieved.
void test_pthread_condattr_getclock(void)
{
    fprintf(stderr, "testing pthread_condattr_getclock() ... ");

    pthread_condattr_t attr = {
        .clock = CLOCK_MONOTONIC,
    };
    int ret = pthread_condattr_init(&attr);
    assert(ret == 0);

    clockid_t clock_id = CLOCK_MONOTONIC;
    ret = pthread_condattr_getclock(&attr, &clock_id);
    assert(ret == 0);
    assert(clock_id == CLOCK_REALTIME);

    // The stored clock attribute must be returned instead of a hard-coded default.
    attr.clock = CLOCK_MONOTONIC;
    ret = pthread_condattr_getclock(&attr, &clock_id);
    assert(ret == 0);
    assert(clock_id == CLOCK_MONOTONIC);

    // Invalid pointers must be rejected.
    ret = pthread_condattr_getclock(NULL, &clock_id);
    assert(ret != 0);
    ret = pthread_condattr_getclock(&attr, NULL);
    assert(ret != 0);

    // Misaligned pointers must be rejected.
    _Alignas(pthread_condattr_t) unsigned char attr_storage[sizeof(pthread_condattr_t) + 1];
    ret = pthread_condattr_getclock((pthread_condattr_t *)&attr_storage[1], &clock_id);
    assert(ret != 0);
    _Alignas(clockid_t) unsigned char clock_id_storage[sizeof(clockid_t) + 1];
    ret = pthread_condattr_getclock(&attr, (clockid_t *)&clock_id_storage[1]);
    assert(ret != 0);

    // Uninitialized condition variable attributes objects must be rejected.
    attr.is_initialized = 0;
    ret = pthread_condattr_getclock(&attr, &clock_id);
    assert(ret != 0);

    fprintf(stderr, "passed\n");
}

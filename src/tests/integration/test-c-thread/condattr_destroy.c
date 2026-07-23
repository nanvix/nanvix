/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <errno.h>
#include <pthread.h>
#include <stdio.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests if condition variable attributes can be destroyed.
void test_pthread_condattr_destroy(void)
{
    fprintf(stderr, "testing pthread_condattr_destroy() ... ");

    pthread_condattr_t attr = {
        0,
    };
    int ret = pthread_condattr_init(&attr);
    assert(ret == 0);
    assert(attr.is_initialized != 0);

    // Destroying an initialized condition variable attributes object must invalidate it.
    ret = pthread_condattr_destroy(&attr);
    assert(ret == 0);
    assert(attr.is_initialized == 0);

    // Destroying an uninitialized condition variable attributes object must fail.
    ret = pthread_condattr_destroy(&attr);
    assert(ret == EINVAL);

    // Invalid pointers must be rejected.
    ret = pthread_condattr_destroy(NULL);
    assert(ret == EINVAL);

    // Misaligned pointers must be rejected.
    _Alignas(pthread_condattr_t) unsigned char attr_storage[sizeof(pthread_condattr_t) + 1];
    ret = pthread_condattr_destroy((pthread_condattr_t *)&attr_storage[1]);
    assert(ret == EINVAL);

    // A destroyed condition variable attributes object may be initialized again.
    ret = pthread_condattr_init(&attr);
    assert(ret == 0);
    assert(attr.is_initialized != 0);
    ret = pthread_condattr_destroy(&attr);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

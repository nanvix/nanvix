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

// Tests if mutex attributes can be destroyed.
void test_pthread_mutexattr_destroy(void)
{
    fprintf(stderr, "testing pthread_mutexattr_destroy() ... ");

    pthread_mutexattr_t attr = {
        0,
    };
    int ret = pthread_mutexattr_init(&attr);
    assert(ret == 0);
    assert(attr.is_initialized != 0);

    // Destroying an initialized mutex attributes object must invalidate it.
    ret = pthread_mutexattr_destroy(&attr);
    assert(ret == 0);
    assert(attr.is_initialized == 0);

    // Destroying an uninitialized mutex attributes object must fail.
    ret = pthread_mutexattr_destroy(&attr);
    assert(ret == EINVAL);

    // Invalid pointers must be rejected.
    ret = pthread_mutexattr_destroy(NULL);
    assert(ret == EINVAL);

    // A destroyed mutex attributes object may be initialized again.
    ret = pthread_mutexattr_init(&attr);
    assert(ret == 0);
    assert(attr.is_initialized != 0);
    ret = pthread_mutexattr_destroy(&attr);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

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

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests if the process-sharing attribute can be retrieved.
void test_pthread_mutexattr_getpshared(void)
{
    fprintf(stderr, "testing pthread_mutexattr_getpshared() ... ");

    pthread_mutexattr_t attr = {
        .pshared = PTHREAD_PROCESS_SHARED,
    };
    int ret = pthread_mutexattr_init(&attr);
    assert(ret == 0);

    // Initialization must overwrite caller-provided storage with the default value.
    assert(attr.pshared == PTHREAD_PROCESS_PRIVATE);

    int pshared = PTHREAD_PROCESS_SHARED;
    ret = pthread_mutexattr_getpshared(&attr, &pshared);
    assert(ret == 0);
    assert(pshared == PTHREAD_PROCESS_PRIVATE);

    // Invalid pointers must be rejected.
    ret = pthread_mutexattr_getpshared(NULL, &pshared);
    assert(ret != 0);
    ret = pthread_mutexattr_getpshared(&attr, NULL);
    assert(ret != 0);

    ret = pthread_mutexattr_destroy(&attr);
    assert(ret == 0);

    // Uninitialized mutex attributes objects must be rejected.
    ret = pthread_mutexattr_getpshared(&attr, &pshared);
    assert(ret != 0);

    fprintf(stderr, "passed\n");
}

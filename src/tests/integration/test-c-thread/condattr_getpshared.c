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
void test_pthread_condattr_getpshared(void)
{
    fprintf(stderr, "testing pthread_condattr_getpshared() ... ");

    pthread_condattr_t attr = {
        .pshared = PTHREAD_PROCESS_SHARED,
    };
    int ret = pthread_condattr_init(&attr);
    assert(ret == 0);

    // Initialization must overwrite caller-provided storage with the default value.
    assert(attr.pshared == PTHREAD_PROCESS_PRIVATE);

    int pshared = PTHREAD_PROCESS_SHARED;
    ret = pthread_condattr_getpshared(&attr, &pshared);
    assert(ret == 0);
    assert(pshared == PTHREAD_PROCESS_PRIVATE);

    // Invalid pointers must be rejected.
    ret = pthread_condattr_getpshared(NULL, &pshared);
    assert(ret != 0);
    ret = pthread_condattr_getpshared(&attr, NULL);
    assert(ret != 0);

    // Misaligned pointers must be rejected.
    _Alignas(pthread_condattr_t) unsigned char attr_storage[sizeof(pthread_condattr_t) + 1];
    ret = pthread_condattr_getpshared((pthread_condattr_t *)&attr_storage[1], &pshared);
    assert(ret != 0);
    _Alignas(int) unsigned char pshared_storage[sizeof(int) + 1];
    ret = pthread_condattr_getpshared(&attr, (int *)&pshared_storage[1]);
    assert(ret != 0);

    // Uninitialized condition variable attributes objects must be rejected.
    attr.is_initialized = 0;
    ret = pthread_condattr_getpshared(&attr, &pshared);
    assert(ret != 0);

    fprintf(stderr, "passed\n");
}

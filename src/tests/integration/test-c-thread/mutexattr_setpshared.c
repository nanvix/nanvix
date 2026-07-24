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
#include <string.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests if the process-sharing attribute can be set and retrieved.
void test_pthread_mutexattr_setpshared(void)
{
    fprintf(stderr, "testing pthread_mutexattr_setpshared() ... ");

    pthread_mutexattr_t attr = {
        0,
    };
    int ret = pthread_mutexattr_init(&attr);
    assert(ret == 0);

    ret = pthread_mutexattr_setpshared(&attr, PTHREAD_PROCESS_PRIVATE);
    assert(ret == 0);

    int pshared = PTHREAD_PROCESS_SHARED;
    ret = pthread_mutexattr_getpshared(&attr, &pshared);
    assert(ret == 0);
    assert(pshared == PTHREAD_PROCESS_PRIVATE);

    // Process-shared mutexes are not supported.
    ret = pthread_mutexattr_setpshared(&attr, PTHREAD_PROCESS_SHARED);
    assert(ret == ENOTSUP);
    ret = pthread_mutexattr_getpshared(&attr, &pshared);
    assert(ret == 0);
    assert(pshared == PTHREAD_PROCESS_PRIVATE);

    // Invalid values must not change the attribute.
    ret = pthread_mutexattr_setpshared(&attr, -1);
    assert(ret == EINVAL);
    ret = pthread_mutexattr_getpshared(&attr, &pshared);
    assert(ret == 0);
    assert(pshared == PTHREAD_PROCESS_PRIVATE);

    // Invalid pointers must be rejected.
    ret = pthread_mutexattr_setpshared(NULL, PTHREAD_PROCESS_PRIVATE);
    assert(ret == EINVAL);
    ret = pthread_mutexattr_getpshared(&attr, &pshared);
    assert(ret == 0);
    assert(pshared == PTHREAD_PROCESS_PRIVATE);

    // Misaligned pointers must be rejected.
    _Alignas(pthread_mutexattr_t) unsigned char attr_storage[sizeof(pthread_mutexattr_t) + 1];
    const unsigned char sentinel = 0xa5;
    memset(attr_storage, sentinel, sizeof(attr_storage));
    unsigned char attr_storage_before[sizeof(attr_storage)];
    memcpy(attr_storage_before, attr_storage, sizeof(attr_storage));
    ret = pthread_mutexattr_setpshared((pthread_mutexattr_t *)&attr_storage[1],
                                      PTHREAD_PROCESS_PRIVATE);
    assert(ret == EINVAL);
    assert(memcmp(attr_storage, attr_storage_before, sizeof(attr_storage)) == 0);

    // Uninitialized mutex attributes objects must be rejected.
    ret = pthread_mutexattr_destroy(&attr);
    assert(ret == 0);
    assert(attr.is_initialized == 0);
    assert(attr.pshared == PTHREAD_PROCESS_PRIVATE);
    ret = pthread_mutexattr_setpshared(&attr, PTHREAD_PROCESS_PRIVATE);
    assert(ret == EINVAL);
    assert(attr.is_initialized == 0);
    assert(attr.pshared == PTHREAD_PROCESS_PRIVATE);

    fprintf(stderr, "passed\n");
}

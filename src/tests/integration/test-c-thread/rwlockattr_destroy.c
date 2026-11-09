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

// Tests if read-write lock attributes can be destroyed.
void test_pthread_rwlockattr_destroy(void)
{
    fprintf(stderr, "testing pthread_rwlockattr_destroy() ... ");

    pthread_rwlockattr_t attr = {
        0,
    };
    int ret = pthread_rwlockattr_init(&attr);
    assert(ret == 0);
    assert(attr.is_initialized != 0);

    // Destroying an initialized read-write lock attributes object must invalidate it.
    ret = pthread_rwlockattr_destroy(&attr);
    assert(ret == 0);
    assert(attr.is_initialized == 0);

    // Destroying an uninitialized read-write lock attributes object must fail.
    ret = pthread_rwlockattr_destroy(&attr);
    assert(ret == EINVAL);
    assert(attr.is_initialized == 0);

    // Invalid pointers must be rejected.
    ret = pthread_rwlockattr_destroy(NULL);
    assert(ret == EINVAL);

    // Misaligned pointers must be rejected without touching the underlying storage. The storage
    // holds non-zero bytes, so it would look initialized if the alignment check were skipped.
    _Alignas(pthread_rwlockattr_t) unsigned char attr_storage[sizeof(pthread_rwlockattr_t) + 1];
    const unsigned char sentinel = 0xa5;
    memset(attr_storage, sentinel, sizeof(attr_storage));
    unsigned char attr_storage_before[sizeof(attr_storage)];
    memcpy(attr_storage_before, attr_storage, sizeof(attr_storage));
    ret = pthread_rwlockattr_destroy((pthread_rwlockattr_t *)&attr_storage[1]);
    assert(ret == EINVAL);
    assert(memcmp(attr_storage, attr_storage_before, sizeof(attr_storage)) == 0);

    // A destroyed read-write lock attributes object may be initialized again.
    ret = pthread_rwlockattr_init(&attr);
    assert(ret == 0);
    assert(attr.is_initialized != 0);
    assert(attr.pshared == PTHREAD_PROCESS_PRIVATE);
    ret = pthread_rwlockattr_destroy(&attr);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

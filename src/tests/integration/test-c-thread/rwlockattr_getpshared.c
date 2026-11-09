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

// Tests if the process-sharing attribute of read-write lock attributes can be retrieved.
void test_pthread_rwlockattr_getpshared(void)
{
    fprintf(stderr, "testing pthread_rwlockattr_getpshared() ... ");

    pthread_rwlockattr_t attr = {
        .pshared = PTHREAD_PROCESS_SHARED,
    };
    int ret = pthread_rwlockattr_init(&attr);
    assert(ret == 0);

    // Initialization must overwrite caller-provided storage with the default value.
    assert(attr.pshared == PTHREAD_PROCESS_PRIVATE);

    int pshared = PTHREAD_PROCESS_SHARED;
    ret = pthread_rwlockattr_getpshared(&attr, &pshared);
    assert(ret == 0);
    assert(pshared == PTHREAD_PROCESS_PRIVATE);

    // Invalid pointers must be rejected without writing the output.
    pshared = -1;
    ret = pthread_rwlockattr_getpshared(NULL, &pshared);
    assert(ret == EINVAL);
    assert(pshared == -1);
    ret = pthread_rwlockattr_getpshared(&attr, NULL);
    assert(ret == EINVAL);

    // Misaligned pointers must be rejected without writing the output. The attributes storage
    // holds non-zero bytes, so it would look initialized if the alignment check were skipped.
    const unsigned char sentinel = 0xa5;
    _Alignas(pthread_rwlockattr_t) unsigned char attr_storage[sizeof(pthread_rwlockattr_t) + 1];
    memset(attr_storage, sentinel, sizeof(attr_storage));
    ret = pthread_rwlockattr_getpshared((pthread_rwlockattr_t *)&attr_storage[1], &pshared);
    assert(ret == EINVAL);
    assert(pshared == -1);
    _Alignas(int) unsigned char pshared_storage[sizeof(int) + 1];
    memset(pshared_storage, sentinel, sizeof(pshared_storage));
    unsigned char pshared_storage_before[sizeof(pshared_storage)];
    memcpy(pshared_storage_before, pshared_storage, sizeof(pshared_storage));
    ret = pthread_rwlockattr_getpshared(&attr, (int *)&pshared_storage[1]);
    assert(ret == EINVAL);
    assert(memcmp(pshared_storage, pshared_storage_before, sizeof(pshared_storage)) == 0);

    // Uninitialized read-write lock attributes objects must be rejected.
    ret = pthread_rwlockattr_destroy(&attr);
    assert(ret == 0);
    ret = pthread_rwlockattr_getpshared(&attr, &pshared);
    assert(ret == EINVAL);
    assert(pshared == -1);

    fprintf(stderr, "passed\n");
}

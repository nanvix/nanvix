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

// Tests if read-write lock attributes can be initialized.
void test_pthread_rwlockattr_init(void)
{
    fprintf(stderr, "testing pthread_rwlockattr_init() ... ");

    pthread_rwlockattr_t attr = {
        .is_initialized = 0,
        .pshared = PTHREAD_PROCESS_SHARED,
    };
    int ret = pthread_rwlockattr_init(&attr);
    assert(ret == 0);

    // Initialization must overwrite caller-provided storage with default values.
    assert(attr.is_initialized != 0);
    assert(attr.pshared == PTHREAD_PROCESS_PRIVATE);

    // Invalid pointers must be rejected.
    ret = pthread_rwlockattr_init(NULL);
    assert(ret == EINVAL);

    // Misaligned pointers must be rejected without touching the underlying storage.
    _Alignas(pthread_rwlockattr_t) unsigned char attr_storage[sizeof(pthread_rwlockattr_t) + 1];
    const unsigned char sentinel = 0xa5;
    memset(attr_storage, sentinel, sizeof(attr_storage));
    unsigned char attr_storage_before[sizeof(attr_storage)];
    memcpy(attr_storage_before, attr_storage, sizeof(attr_storage));
    ret = pthread_rwlockattr_init((pthread_rwlockattr_t *)&attr_storage[1]);
    assert(ret == EINVAL);
    assert(memcmp(attr_storage, attr_storage_before, sizeof(attr_storage)) == 0);

    ret = pthread_rwlockattr_destroy(&attr);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

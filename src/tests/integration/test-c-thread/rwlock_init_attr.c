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

// Tests if read-write locks can be initialized with an attributes object.
void test_pthread_rwlock_init_attr(void)
{
    fprintf(stderr, "testing pthread_rwlock_init() with attributes ... ");

    pthread_rwlockattr_t attr = {
        0,
    };
    int ret = pthread_rwlockattr_init(&attr);
    assert(ret == 0);
    ret = pthread_rwlockattr_setpshared(&attr, PTHREAD_PROCESS_PRIVATE);
    assert(ret == 0);

    // A single attributes object may initialize multiple read-write locks.
    pthread_rwlock_t rwlock1 = 0;
    pthread_rwlock_t rwlock2 = 0;
    ret = pthread_rwlock_init(&rwlock1, &attr);
    assert(ret == 0);
    ret = pthread_rwlock_init(&rwlock2, &attr);
    assert(ret == 0);

    // Destroying the attributes object must not affect read-write locks initialized with it.
    ret = pthread_rwlockattr_destroy(&attr);
    assert(ret == 0);
    assert(pthread_rwlock_rdlock(&rwlock1) == 0);
    assert(pthread_rwlock_unlock(&rwlock1) == 0);
    assert(pthread_rwlock_wrlock(&rwlock2) == 0);
    assert(pthread_rwlock_unlock(&rwlock2) == 0);
    assert(pthread_rwlock_destroy(&rwlock1) == 0);
    assert(pthread_rwlock_destroy(&rwlock2) == 0);

    // Uninitialized attributes objects must be rejected.
    pthread_rwlock_t rwlock = 0;
    ret = pthread_rwlock_init(&rwlock, &attr);
    assert(ret == EINVAL);

    // Process-shared read-write locks are not supported.
    ret = pthread_rwlockattr_init(&attr);
    assert(ret == 0);
    attr.pshared = PTHREAD_PROCESS_SHARED;
    ret = pthread_rwlock_init(&rwlock, &attr);
    assert(ret == ENOTSUP);

    // Invalid process-sharing attributes must be rejected.
    attr.pshared = -1;
    ret = pthread_rwlock_init(&rwlock, &attr);
    assert(ret == EINVAL);

    // Misaligned attributes pointers must be rejected.
    attr.pshared = PTHREAD_PROCESS_PRIVATE;
    _Alignas(pthread_rwlockattr_t) unsigned char attr_storage[sizeof(pthread_rwlockattr_t) + 1];
    memcpy(&attr_storage[1], &attr, sizeof(attr));
    ret = pthread_rwlock_init(&rwlock, (const pthread_rwlockattr_t *)&attr_storage[1]);
    assert(ret == EINVAL);

    // Failed initializations must leave the read-write lock uninitialized.
    ret = pthread_rwlock_destroy(&rwlock);
    assert(ret == EINVAL);

    // A valid attributes object must still initialize the read-write lock.
    ret = pthread_rwlock_init(&rwlock, &attr);
    assert(ret == 0);
    assert(pthread_rwlock_wrlock(&rwlock) == 0);
    assert(pthread_rwlock_unlock(&rwlock) == 0);
    assert(pthread_rwlock_destroy(&rwlock) == 0);

    ret = pthread_rwlockattr_destroy(&attr);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

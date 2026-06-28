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
// Constants
//==================================================================================================

// A mutex type that is not supported by the implementation.
#define INVALID_MUTEX_TYPE 1000

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Asserts that a mutex type can be set and read back from a mutex attributes object.
static void assert_roundtrip(pthread_mutexattr_t *attr, int wanted)
{
    int got = -1;
    int ret = pthread_mutexattr_settype(attr, wanted);
    assert(ret == 0);
    ret = pthread_mutexattr_gettype(attr, &got);
    assert(ret == 0);
    assert(got == wanted);

    // Setting the type must mirror PTHREAD_MUTEX_RECURSIVE onto the recursive flag.
    assert(attr->recursive == (wanted == PTHREAD_MUTEX_RECURSIVE));
}

// Tests if the mutex type attribute can be set and retrieved.
void test_pthread_mutexattr_settype(void)
{
    fprintf(stderr, "testing pthread_mutexattr_settype() and pthread_mutexattr_gettype() ... ");

    // Initialize the mutex attributes object and assert operation.
    pthread_mutexattr_t attr = {
        .type = INVALID_MUTEX_TYPE,
        .recursive = 1,
    };
    int ret = pthread_mutexattr_init(&attr);
    assert(ret == 0);

    // Initialization must overwrite caller-provided storage with default values.
    assert(attr.is_initialized != 0);
    assert(attr.recursive == 0);

    int type = -1;
    ret = pthread_mutexattr_gettype(&attr, &type);
    assert(ret == 0);
    assert(type == PTHREAD_MUTEX_DEFAULT);

    // Each supported mutex type must round-trip through set/get.
    assert_roundtrip(&attr, PTHREAD_MUTEX_NORMAL);
    assert_roundtrip(&attr, PTHREAD_MUTEX_RECURSIVE);
    assert_roundtrip(&attr, PTHREAD_MUTEX_ERRORCHECK);
    assert_roundtrip(&attr, PTHREAD_MUTEX_DEFAULT);

    // An unsupported type must be rejected and must leave the stored type unchanged.
    ret = pthread_mutexattr_settype(&attr, INVALID_MUTEX_TYPE);
    assert(ret != 0);
    ret = pthread_mutexattr_gettype(&attr, &type);
    assert(ret == 0);
    assert(type == PTHREAD_MUTEX_DEFAULT);

    // Invalid pointers must be rejected.
    ret = pthread_mutexattr_init(NULL);
    assert(ret != 0);
    ret = pthread_mutexattr_settype(NULL, PTHREAD_MUTEX_NORMAL);
    assert(ret != 0);
    ret = pthread_mutexattr_gettype(NULL, &type);
    assert(ret != 0);
    ret = pthread_mutexattr_gettype(&attr, NULL);
    assert(ret != 0);

    // Destroy the mutex attributes object and assert operation.
    ret = pthread_mutexattr_destroy(&attr);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

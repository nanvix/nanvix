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

// Worker thread.
static void *worker_thread(void *arg)
{
    assert(arg == NULL);

    // New threads start with the deferred type, regardless of the type of their creator.
    int oldtype = -1;
    int ret = pthread_setcanceltype(PTHREAD_CANCEL_ASYNCHRONOUS, &oldtype);
    assert(ret == 0);
    assert(oldtype == PTHREAD_CANCEL_DEFERRED);

    ret = pthread_setcanceltype(PTHREAD_CANCEL_DEFERRED, NULL);
    assert(ret == 0);

    return NULL;
}

// Tests if the cancellation type of the calling thread can be changed.
void test_pthread_setcanceltype(void)
{
    fprintf(stderr, "testing pthread_setcanceltype() ... ");

    // The main thread starts with the deferred type.
    int oldtype = -1;
    int ret = pthread_setcanceltype(PTHREAD_CANCEL_ASYNCHRONOUS, &oldtype);
    assert(ret == 0);
    assert(oldtype == PTHREAD_CANCEL_DEFERRED);

    pthread_t worker_tid = PTHREAD_NULL;
    ret = pthread_create(&worker_tid, NULL, worker_thread, NULL);
    assert(ret == 0);
    ret = pthread_join(worker_tid, NULL);
    assert(ret == 0);

    // The type is per thread, so the worker thread did not change the type of the main thread.
    oldtype = -1;
    ret = pthread_setcanceltype(PTHREAD_CANCEL_DEFERRED, &oldtype);
    assert(ret == 0);
    assert(oldtype == PTHREAD_CANCEL_ASYNCHRONOUS);

    // An invalid type fails without storing the old type.
    oldtype = -1;
    ret = pthread_setcanceltype(-1, &oldtype);
    assert(ret == EINVAL);
    assert(oldtype == -1);

    // The type and the state are independent, and the invalid type above left the type unchanged.
    int prevstate = -1;
    ret = pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &prevstate);
    assert(ret == 0);
    oldtype = -1;
    ret = pthread_setcanceltype(PTHREAD_CANCEL_ASYNCHRONOUS, &oldtype);
    assert(ret == 0);
    assert(oldtype == PTHREAD_CANCEL_DEFERRED);
    int oldstate = -1;
    ret = pthread_setcancelstate(prevstate, &oldstate);
    assert(ret == 0);
    assert(oldstate == PTHREAD_CANCEL_DISABLE);

    // The old type storage may be null.
    ret = pthread_setcanceltype(PTHREAD_CANCEL_DEFERRED, NULL);
    assert(ret == 0);
    oldtype = -1;
    ret = pthread_setcanceltype(PTHREAD_CANCEL_DEFERRED, &oldtype);
    assert(ret == 0);
    assert(oldtype == PTHREAD_CANCEL_DEFERRED);

    fprintf(stderr, "passed\n");
}

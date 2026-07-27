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

    int oldstate = -1;
    int ret = pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &oldstate);
    assert(ret == 0);
    assert(oldstate == PTHREAD_CANCEL_ENABLE);

    ret = pthread_setcancelstate(PTHREAD_CANCEL_ENABLE, NULL);
    assert(ret == 0);

    return NULL;
}

// Tests if the cancellation state of the calling thread can be changed.
void test_pthread_setcancelstate(void)
{
    fprintf(stderr, "testing pthread_setcancelstate() ... ");

    int oldstate = -1;
    int ret = pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &oldstate);
    assert(ret == 0);
    assert(oldstate == PTHREAD_CANCEL_ENABLE);

    pthread_t worker_tid = PTHREAD_NULL;
    ret = pthread_create(&worker_tid, NULL, worker_thread, NULL);
    assert(ret == 0);
    ret = pthread_join(worker_tid, NULL);
    assert(ret == 0);

    oldstate = -1;
    ret = pthread_setcancelstate(PTHREAD_CANCEL_ENABLE, &oldstate);
    assert(ret == 0);
    assert(oldstate == PTHREAD_CANCEL_DISABLE);

    ret = pthread_setcancelstate(-1, &oldstate);
    assert(ret == EINVAL);
    assert(oldstate == PTHREAD_CANCEL_DISABLE);

    oldstate = -1;
    ret = pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &oldstate);
    assert(ret == 0);
    assert(oldstate == PTHREAD_CANCEL_ENABLE);

    ret = pthread_setcancelstate(PTHREAD_CANCEL_ENABLE, NULL);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

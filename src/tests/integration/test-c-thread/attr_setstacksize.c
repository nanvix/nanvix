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
#include <sched.h>
#include <stdio.h>

//==================================================================================================
// Constants
//==================================================================================================

#define STACK_SIZE_DELTA 4096

//==================================================================================================
// Global Variables
//==================================================================================================

// Signals that the worker may inspect its attributes after pthread_create() records its stack.
static volatile int worker_may_run = 0;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Checks that the worker thread was created with the requested stack size.
static void *worker_thread(void *arg)
{
    while (!worker_may_run) {
        int ret = sched_yield();
        assert(ret == 0);
    }

    size_t expected_stacksize = *(const size_t *)arg;
    pthread_attr_t attr = {
        0,
    };
    int ret = pthread_getattr_np(pthread_self(), &attr);
    assert(ret == 0);

    size_t actual_stacksize = 0;
    ret = pthread_attr_getstacksize(&attr, &actual_stacksize);
    assert(ret == 0);
    assert(actual_stacksize == expected_stacksize);

    ret = pthread_attr_destroy(&attr);
    assert(ret == 0);

    return NULL;
}

// Tests if pthread_attr_setstacksize() validates, stores, and applies the stack size attribute.
void test_pthread_attr_setstacksize(void)
{
    fprintf(stderr, "testing pthread_attr_setstacksize() ... ");

    pthread_attr_t attr = {
        0,
    };
    int ret = pthread_attr_init(&attr);
    assert(ret == 0);

    size_t default_stacksize = 0;
    ret = pthread_attr_getstacksize(&attr, &default_stacksize);
    assert(ret == 0);

    size_t stacksize = default_stacksize + STACK_SIZE_DELTA;
    assert(stacksize > default_stacksize);
    ret = pthread_attr_setstacksize(&attr, stacksize);
    assert(ret == 0);

    size_t actual_stacksize = 0;
    ret = pthread_attr_getstacksize(&attr, &actual_stacksize);
    assert(ret == 0);
    assert(actual_stacksize == stacksize);

    ret = pthread_attr_setstacksize(NULL, stacksize);
    assert(ret == EINVAL);
    pthread_attr_t uninitialized_attr = {
        0,
    };
    ret = pthread_attr_setstacksize(&uninitialized_attr, stacksize);
    assert(ret == EINVAL);
    ret = pthread_attr_setstacksize(&attr, 0);
    assert(ret == EINVAL);

    ret = pthread_attr_getstacksize(&attr, &actual_stacksize);
    assert(ret == 0);
    assert(actual_stacksize == stacksize);

    worker_may_run = 0;
    pthread_t worker_tid = PTHREAD_NULL;
    ret = pthread_create(&worker_tid, &attr, worker_thread, &stacksize);
    assert(ret == 0);
    assert(worker_tid != PTHREAD_NULL);

    worker_may_run = 1;
    ret = pthread_join(worker_tid, NULL);
    assert(ret == 0);

    ret = pthread_attr_destroy(&attr);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

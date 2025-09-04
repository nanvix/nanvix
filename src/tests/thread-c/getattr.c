/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#define _GNU_SOURCE 1
#include <assert.h>
#include <pthread.h>
#include <stdio.h>

/* FIXME: Remove the following import once it is available from NewLib headers. */
extern int pthread_getattr_np(pthread_t thread, pthread_attr_t *attr);

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests if pthread_getattr_np() can retrieve attributes and they can be destroyed.
void test_pthread_getattr_np_destroy(void)
{
    fprintf(stderr, "testing pthread_getattr_np() and pthread_attr_destroy() ... ");

    // Get attributes of the calling thread.
    pthread_attr_t attr = {0};
    int ret = pthread_getattr_np(pthread_self(), &attr);
    assert(ret == 0);

    // Sanity check a couple of fields via accessors; values are implementation-defined,
    // so we only verify that getters succeed and return something sensible.
    size_t stacksize = 0;
    void *stackaddr = NULL;
    ret = pthread_attr_getstack(&attr, &stackaddr, &stacksize);
    assert(ret == 0);
    assert(stacksize > 0);

    int detachstate = 0;
    ret = pthread_attr_getdetachstate(&attr, &detachstate);
    assert(ret == 0);
    assert(detachstate == PTHREAD_CREATE_JOINABLE || detachstate == PTHREAD_CREATE_DETACHED);

    // Destroy the attributes object.
    ret = pthread_attr_destroy(&attr);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

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

// Tests if the detach state attribute can be set and retrieved.
void test_pthread_attr_setdetachstate(void)
{
    fprintf(stderr, "testing pthread_attr_setdetachstate() ... ");

    pthread_attr_t attr = {
        0,
    };
    int ret = pthread_attr_init(&attr);
    assert(ret == 0);

    int detachstate = -1;
    ret = pthread_attr_getdetachstate(&attr, &detachstate);
    assert(ret == 0);
    assert(detachstate == PTHREAD_CREATE_JOINABLE);

    ret = pthread_attr_setdetachstate(&attr, PTHREAD_CREATE_DETACHED);
    assert(ret == 0);
    ret = pthread_attr_getdetachstate(&attr, &detachstate);
    assert(ret == 0);
    assert(detachstate == PTHREAD_CREATE_DETACHED);

    ret = pthread_attr_setdetachstate(&attr, PTHREAD_CREATE_JOINABLE);
    assert(ret == 0);
    ret = pthread_attr_getdetachstate(&attr, &detachstate);
    assert(ret == 0);
    assert(detachstate == PTHREAD_CREATE_JOINABLE);

    ret = pthread_attr_setdetachstate(&attr, -1);
    assert(ret == EINVAL);
    ret = pthread_attr_getdetachstate(&attr, &detachstate);
    assert(ret == 0);
    assert(detachstate == PTHREAD_CREATE_JOINABLE);

    ret = pthread_attr_destroy(&attr);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

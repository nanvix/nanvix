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
// Standalone Functions
//==================================================================================================

// Tests if scheduling parameters can be set and retrieved.
void test_pthread_attr_setschedparam(void)
{
    fprintf(stderr, "testing pthread_attr_setschedparam() ... ");

    pthread_attr_t attr = {
        0,
    };
    int ret = pthread_attr_init(&attr);
    assert(ret == 0);

    struct sched_param param = {
        .sched_priority = -1,
    };
    ret = pthread_attr_getschedparam(&attr, &param);
    assert(ret == 0);
    assert(param.sched_priority == 0);

    param.sched_priority = 42;
    ret = pthread_attr_setschedparam(&attr, &param);
    assert(ret == 0);

    param.sched_priority = -1;
    ret = pthread_attr_getschedparam(&attr, &param);
    assert(ret == 0);
    assert(param.sched_priority == 42);

    ret = pthread_attr_destroy(&attr);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

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
#include <stdlib.h>

//==================================================================================================
// Constants
//==================================================================================================

#define STACK_ALIGNMENT 16

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests if pthread_attr_setstack() validates and stores stack attributes.
void test_pthread_attr_setstack(void)
{
    fprintf(stderr, "testing pthread_attr_setstack() ... ");

    pthread_attr_t attr = {
        0,
    };
    int ret = pthread_attr_init(&attr);
    assert(ret == 0);

    void *default_stackaddr = NULL;
    size_t default_stacksize = 0;
    ret = pthread_attr_getstack(&attr, &default_stackaddr, &default_stacksize);
    assert(ret == 0);

    size_t stacksize = default_stacksize + STACK_ALIGNMENT;
    void *stackaddr = malloc(stacksize);
    assert(stackaddr != NULL);

    ret = pthread_attr_setstack(&attr, stackaddr, stacksize);
    assert(ret == 0);

    void *actual_stackaddr = NULL;
    size_t actual_stacksize = 0;
    ret = pthread_attr_getstack(&attr, &actual_stackaddr, &actual_stacksize);
    assert(ret == 0);
    assert(actual_stackaddr == stackaddr);
    assert(actual_stacksize == stacksize);

    ret = pthread_attr_setstack(NULL, stackaddr, stacksize);
    assert(ret == EINVAL);
    pthread_attr_t uninitialized_attr = {
        0,
    };
    ret = pthread_attr_setstack(&uninitialized_attr, stackaddr, stacksize);
    assert(ret == EINVAL);
    ret = pthread_attr_setstack(&attr, NULL, stacksize);
    assert(ret == EINVAL);
    ret = pthread_attr_setstack(&attr, stackaddr, 0);
    assert(ret == EINVAL);
    ret = pthread_attr_setstack(&attr, (char *)stackaddr + 1, stacksize - 1);
    assert(ret == EINVAL);
    ret = pthread_attr_setstack(&attr, stackaddr, stacksize - 1);
    assert(ret == EINVAL);

    ret = pthread_attr_getstack(&attr, &actual_stackaddr, &actual_stacksize);
    assert(ret == 0);
    assert(actual_stackaddr == stackaddr);
    assert(actual_stacksize == stacksize);

    ret = pthread_attr_destroy(&attr);
    assert(ret == 0);
    free(stackaddr);

    fprintf(stderr, "passed\n");
}

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

// Tests if the guard size attribute can be retrieved.
void test_pthread_attr_getguardsize(void)
{
    fprintf(stderr, "testing pthread_attr_getguardsize() ... ");

    pthread_attr_t attr = {
        0,
    };
    int ret = pthread_attr_init(&attr);
    assert(ret == 0);

    size_t guardsize = 0;
    ret = pthread_attr_getguardsize(&attr, &guardsize);
    assert(ret == 0);
    assert(guardsize == 0);

    guardsize = 1;
    ret = pthread_attr_getguardsize(&attr, &guardsize);
    assert(ret == 0);
    assert(guardsize == 0);

    ret = pthread_attr_destroy(&attr);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

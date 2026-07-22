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

// Tests if the guard size attribute can be set and retrieved.
void test_pthread_attr_setguardsize(void)
{
    fprintf(stderr, "testing pthread_attr_setguardsize() ... ");

    pthread_attr_t attr = {
        0,
    };
    int ret = pthread_attr_init(&attr);
    assert(ret == 0);

    size_t guardsize = 0;
    ret = pthread_attr_setguardsize(&attr, 4096);
    assert(ret == 0);
    ret = pthread_attr_getguardsize(&attr, &guardsize);
    assert(ret == 0);
    assert(guardsize == 4096);

    ret = pthread_attr_setguardsize(&attr, 0);
    assert(ret == 0);
    ret = pthread_attr_getguardsize(&attr, &guardsize);
    assert(ret == 0);
    assert(guardsize == 0);

    ret = pthread_attr_setguardsize(NULL, 4096);
    assert(ret == EINVAL);

    ret = pthread_attr_destroy(&attr);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

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

// Tests if pthread_attr_getstack() retrieves stack attributes and they can be destroyed.
void test_pthread_attr_getstack(void)
{
    fprintf(stderr, "testing pthread_attr_getstack() and pthread_attr_destroy() ... ");

    // Initialize the thread attributes object.
    pthread_attr_t attr = {0};
    int ret = pthread_attr_init(&attr);
    assert(ret == 0);

    // Retrieve stack attributes.
    size_t stacksize = 0;
    void *stackaddr = NULL;
    ret = pthread_attr_getstack(&attr, &stackaddr, &stacksize);
    assert(ret == 0);

    // Sanity check returned values.
    assert(stacksize > 0);
    assert(stackaddr != NULL);

    // Destroy the attributes object.
    ret = pthread_attr_destroy(&attr);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

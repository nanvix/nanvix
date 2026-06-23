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

// Tests if pthread attributes can be initialized and destroyed.
void test_pthread_attr_init_destroy(void)
{
    fprintf(stderr, "testing pthread_attr_init() and pthread_attr_destroy() ... ");

    // Initialize the thread attributes object and assert operation.
    pthread_attr_t attr = {
        0,
    };
    int ret = pthread_attr_init(&attr);
    assert(ret == 0);

    // Destroy the thread attributes object and assert operation.
    ret = pthread_attr_destroy(&attr);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}

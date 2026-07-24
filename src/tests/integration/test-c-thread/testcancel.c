/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <pthread.h>
#include <stdbool.h>
#include <stdio.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests if a thread can create a cancellation point when no cancellation request is pending.
void test_pthread_testcancel(void)
{
    fprintf(stderr, "testing pthread_testcancel() ... ");

    bool returned = false;
    pthread_testcancel();
    returned = true;
    assert(returned);

    fprintf(stderr, "passed\n");
}

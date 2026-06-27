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

// Main thread.
static void main_thread(void)
{
    // Get the master thread identifier and check if it is valid.
    pthread_t master_tid = pthread_self();
    assert(master_tid != PTHREAD_NULL);
}

// Tests if threads can get their own identifiers.
void test_pthread_self(void)
{
    fprintf(stderr, "testing pthread_self() ... ");

    main_thread();

    fprintf(stderr, "passed\n");
}

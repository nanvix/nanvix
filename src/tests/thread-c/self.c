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
// Constants
//==================================================================================================

// Expected identifier of the master thread.
static const pthread_t EXPECTED_MASTER_TID = 1;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Main thread.
static void main_thread(void)
{
    // Get the master thread identifier and check if it matches the expected value.
    pthread_t master_tid = pthread_self();
    assert(master_tid == EXPECTED_MASTER_TID);
}

// Tests if threads can get their own identifiers.
void test_pthread_self()
{
    fprintf(stderr, "testing pthread_self() ... ");

    main_thread();

    fprintf(stderr, "passed\n");
}

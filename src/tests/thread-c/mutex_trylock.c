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
#include <sched.h>
#include <stdbool.h>
#include <stdio.h>

//==================================================================================================
// Constants
//==================================================================================================

// Expected identifier of the master thread.
static const pthread_t EXPECTED_MASTER_TID = 1;

// Expected argument passed to the worker thread.
static const size_t EXPECTED_WORKER_ARG = 0xbadcafe;

// Expected exit status of the worker thread.
static const size_t EXPECTED_EXIT_STATUS = 0xdeadbeef;

// Dead mutex locked by main thread.
static pthread_mutex_t mutex_main_thread = PTHREAD_MUTEX_INITIALIZER;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Worker thread.
static void *worker_thread(void *arg)
{
    // Check if worker argument matches the expected value.
    assert((size_t)arg == EXPECTED_WORKER_ARG);

    // Try to lock a mutex that is already locked by the main thread.
    assert(pthread_mutex_trylock(&mutex_main_thread) == EBUSY);

    // Exit the worker thread and make sure it returns the expected value.
    return ((void *)EXPECTED_EXIT_STATUS);
}

// Main thread.
static void main_thread(void)
{
    // Get the master thread identifier and check if it matches the expected value.
    pthread_t master_tid = pthread_self();
    assert(master_tid == EXPECTED_MASTER_TID);

    // Lock some resources.
    assert(pthread_mutex_lock(&mutex_main_thread) == 0);

    // Create a worker thread and check if its identifier matches the expected value.
    pthread_t worker_tid = PTHREAD_NULL;
    int ret = pthread_create(&worker_tid, NULL, worker_thread, (void *)EXPECTED_WORKER_ARG);
    assert(ret == 0);
    assert(worker_tid != PTHREAD_NULL);

    // Wait for the worker thread to exit and check if it returns the expected value.
    void *retval = NULL;
    ret = pthread_join(worker_tid, &retval);
    assert(ret == 0);
    assert(retval == (void *)EXPECTED_EXIT_STATUS);

    assert(pthread_mutex_unlock(&mutex_main_thread) == 0);
}

// Tests if calling `pthread_mutex_trylock() works.
void test_pthread_mutex_trylock(void)
{
    fprintf(stderr, "testing pthread_mutex_trylock() ... ");

    main_thread();

    fprintf(stderr, "passed\n");
}

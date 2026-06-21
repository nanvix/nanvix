/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <pthread.h>
#include <sched.h>
#include <stdio.h>

//==================================================================================================
// Constants
//==================================================================================================

// Expected argument passed to the worker thread.
static const size_t EXPECTED_WORKER_ARG = 0xbadcafe;

// Expected exit status of the worker thread.
static const size_t EXPECTED_EXIT_STATUS = 0xdeadbeef;

// Expected thread-specific data for the worker thread.
static const size_t EXPECTED_WORKER_TDA = 0xcafebabe;

// Expected thread-specific data for the main thread.
static const size_t EXPECTED_MAIN_TDA = 0xfeedface;

//==================================================================================================
// Global Variables
//==================================================================================================

// Create a key for thread-specific data.
pthread_key_t key;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Worker thread.
static void *worker_thread(void *arg)
{
    int ret = -1;

    // Check if worker argument matches the expected value.
    assert((size_t)arg == EXPECTED_WORKER_ARG);

    // Set thread-specific data.
    ret = pthread_setspecific(key, (void *)EXPECTED_WORKER_TDA);
    assert(ret == 0);

    // Get thread-specific data and check if it matches the expected value.
    void *value = pthread_getspecific(key);
    assert(value == (void *)EXPECTED_WORKER_TDA);

    // Exit the worker thread and make sure it returns the expected value.
    return ((void *)EXPECTED_EXIT_STATUS);
}

// Main thread.
static void main_thread(void)
{
    int ret = -1;

    ret = pthread_key_create(&key, NULL);
    assert(ret == 0);

    // Set thread-specific data.
    ret = pthread_setspecific(key, (void *)EXPECTED_MAIN_TDA);
    assert(ret == 0);

    // Get thread-specific data and check if it matches the expected value.
    void *value = pthread_getspecific(key);
    assert(value == (void *)EXPECTED_MAIN_TDA);

    // Create a worker thread and check if its identifier matches the expected value.
    pthread_t worker_tid = PTHREAD_NULL;
    ret = pthread_create(&worker_tid, NULL, worker_thread, (void *)EXPECTED_WORKER_ARG);
    assert(ret == 0);
    assert(worker_tid != PTHREAD_NULL);

    // Wait for the worker thread to exit and check if it returns the expected value.
    void *retval = NULL;
    ret = pthread_join(worker_tid, &retval);
    assert(ret == 0);
    assert(retval == (void *)EXPECTED_EXIT_STATUS);

    // Check if thread-specific data for the main thread is still the expected value.
    value = pthread_getspecific(key);
    assert(value == (void *)EXPECTED_MAIN_TDA);

    // Delete the key for thread-specific data.
    ret = pthread_key_delete(key);
    assert(ret == 0);
}

// Tests if thread interface for operating on thread-specific data works.
void test_pthread_tda(void)
{
    fprintf(stderr, "testing pthread_tda() ... ");

    main_thread();

    fprintf(stderr, "passed\n");
}

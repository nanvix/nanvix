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
static pthread_mutex_t dead_mutex_main_thread = PTHREAD_MUTEX_INITIALIZER;

// Dead mutex locked by worker thread.
static pthread_mutex_t dead_mutex_worker_thread = PTHREAD_MUTEX_INITIALIZER;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Worker thread.
static void *worker_thread(void *arg)
{
    // Check if worker argument matches the expected value.
    assert((size_t)arg == EXPECTED_WORKER_ARG);

    // Lock some resources.
    assert(pthread_mutex_lock(&dead_mutex_worker_thread) == 0);

    // Block forever.
    assert(pthread_mutex_lock(&dead_mutex_main_thread) == 0);

    while (true) {
        sched_yield();
    }

    // NOTE: We never gets here.

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
    assert(pthread_mutex_lock(&dead_mutex_main_thread) == 0);

    // Create a worker thread and check if its identifier matches the expected value.
    pthread_t worker_tid = PTHREAD_NULL;
    int ret = pthread_create(&worker_tid, NULL, worker_thread, (void *)EXPECTED_WORKER_ARG);
    assert(ret == 0);
    assert(worker_tid != PTHREAD_NULL);

    // Don't wait for the worker thread to exit, if something goes wrong this test will hang.
}

// Tests if calling exit() causes the program to exit even if there are other threads running.
void test_pthread_nowait(void)
{
    fprintf(stderr, "testing pthread_exit() ... ");

    main_thread();

    fprintf(stderr, "passed\n");
}

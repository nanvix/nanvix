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

//==================================================================================================
// Global Variables
//==================================================================================================

// Global mutex used to synchronize access to the `initialized` variable.
static pthread_mutex_t mutex = {0};

// Global variable used to signal that the worker thread is initialized.
static volatile int initialized = 0;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Worker thread.
static void *worker_thread(void *arg)
{
    // Check if worker argument matches the expected value.
    assert((size_t)arg == EXPECTED_WORKER_ARG);

    // Signal that the worker thread is initialized.
    assert(pthread_mutex_lock(&mutex) == 0);
    initialized = 1;
    assert(pthread_mutex_unlock(&mutex) == 0);

    // Exit the worker thread and make sure it returns the expected value.
    return ((void *)EXPECTED_EXIT_STATUS);
}

// Main thread.
static void main_thread(void)
{
    // Initialize mutex.
    assert(pthread_mutex_init(&mutex, NULL) == 0);

    // Create a worker thread and check if its identifier matches the expected value.
    pthread_t worker_tid = PTHREAD_NULL;
    int ret = pthread_create(&worker_tid, NULL, worker_thread, (void *)EXPECTED_WORKER_ARG);
    assert(ret == 0);
    assert(worker_tid != PTHREAD_NULL);

    // Wait for the worker thread to complete.
    while (1) {
        // Obtain a cached copy of the initialized variable.
        assert(pthread_mutex_lock(&mutex) == 0);
        int initialized_copy = initialized;
        assert(pthread_mutex_unlock(&mutex) == 0);

        if (initialized_copy) {
            break;
        }

        sched_yield();
    }

    // Wait for the worker thread to exit and check if it returns the expected value.
    void *retval = NULL;
    ret = pthread_join(worker_tid, &retval);
    assert(ret == 0);
    assert(retval == (void *)EXPECTED_EXIT_STATUS);

    // Destroy mutex.
    assert(pthread_mutex_destroy(&mutex) == 0);
}

// Tests if dynamically initialized mutexes can be used for synchronization.
void test_pthread_mutex_dynamic_init(void)
{
    fprintf(stderr, "testing pthread_mutex_dynamic_init() ... ");

    for (int i = 0; i < 2; i++) {
        main_thread();
    }

    fprintf(stderr, "passed\n");
}

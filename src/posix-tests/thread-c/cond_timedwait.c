/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <assert.h>
#include <errno.h>
#include <pthread.h>
#include <sched.h>
#include <stdio.h>
#include <time.h>

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

// Global condition variable used for synchronization.
static pthread_cond_t cond = PTHREAD_COND_INITIALIZER;

// Global mutex used to synchronize access to the `initialized` variable.
static pthread_mutex_t mutex = PTHREAD_MUTEX_INITIALIZER;

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

    struct timespec ts = {0};
    assert(clock_gettime(CLOCK_MONOTONIC, &ts) == 0);
    ts.tv_sec += 1;

    // Wait for the main thread to signal the condition variable.
    assert(pthread_mutex_lock(&mutex) == 0);
    assert(pthread_cond_timedwait(&cond, &mutex, &ts) == ETIMEDOUT);
    assert(pthread_mutex_unlock(&mutex) == 0);

    // Exit the worker thread and make sure it returns the expected value.
    return ((void *)EXPECTED_EXIT_STATUS);
}

// Main thread.
static void main_thread(void)
{
    // Create a worker thread and check if its identifier matches the expected value.
    pthread_t worker_tid = PTHREAD_NULL;
    int ret = pthread_create(&worker_tid, NULL, worker_thread, (void *)EXPECTED_WORKER_ARG);
    assert(ret == 0);

    // Never Signal the worker thread.

    // Wait for the worker thread to finish and check its exit status.
    void *exit_status;
    ret = pthread_join(worker_tid, &exit_status);
    assert(ret == 0);
    assert((size_t)exit_status == EXPECTED_EXIT_STATUS);
}

// Tests if `pthread_mutex_cond_timedwait()` can be used for synchronization.
void test_pthread_cond_timedwait(void)
{
    fprintf(stderr, "testing pthread_cond_timedwait() ... ");

    main_thread();

    fprintf(stderr, "passed\n");
}

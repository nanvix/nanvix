/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <assert.h>
#include <pthread.h>
#include <sched.h>

//!
//! This C program tests if mutexes can be used to synchronize access to global variables. It
//! creates a worker thread that writes a magic string to the standard output and then exits. The
//! main thread waits for the worker thread to signal that it is initialized and then waits for the
//! worker thread to exit.
//!

//==================================================================================================
// Constants
//==================================================================================================

// Expected identifier of the master thread.
const pthread_t EXPECTED_MASTER_TID = 1;

// Expected identifier of the worker thread.
const pthread_t EXPECTED_WORKER_TID = 2;

// Expected argument passed to the worker thread.
const size_t EXPECTED_WORKER_ARG = 0xbadcafe;

// Expected exit status of the worker thread.
const size_t EXPECTED_EXIT_STATUS = 0xdeadbeef;

//==================================================================================================
// Global Variables
//==================================================================================================

// Global mutex used to synchronize access to global variables.
static pthread_mutex_t mutex = PTHREAD_MUTEX_INITIALIZER;

// Global variable used to signal that the worker thread is initialized.
static volatile int initialized = 0;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Worker thread.
void *worker(void *arg)
{
    // Check if worker argument matches the expected value.
    assert((size_t)arg == EXPECTED_WORKER_ARG);

    // Get the worker thread identifier and check if it matches the expected value.
    pthread_t worker_tid = pthread_self();
    assert(worker_tid == EXPECTED_WORKER_TID);

    // Signal that the worker thread is initialized.
    pthread_mutex_lock(&mutex);
    initialized = 1;
    pthread_mutex_unlock(&mutex);

    // Exit the worker thread and make sure it returns the expected value.
    return ((void *)EXPECTED_EXIT_STATUS);
}

/**
 * @brief Tests if mutexes can be used to synchronize access to global variables.
 *
 * @param argc Number of command-line arguments (unused).
 * @param argv List of command-line arguments (unused).
 *
 * @returns Always returns zero. If a test fails, the program will abort.
 */
int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    // Get the master thread identifier and check if it matches the expected value.
    pthread_t master_tid = pthread_self();
    assert(master_tid == EXPECTED_MASTER_TID);

    // Create a worker thread and check if its identifier matches the expected value.
    pthread_t worker_tid = 0;
    int ret = pthread_create(&worker_tid, NULL, worker, (void *)EXPECTED_WORKER_ARG);
    assert(ret == 0);
    assert(worker_tid == EXPECTED_WORKER_TID);

    // Wait for the worker thread to complete.
    while (1) {
        // Obtain a cached copy of the initialized variable.
        pthread_mutex_lock(&mutex);
        int initialized_copy = initialized;
        pthread_mutex_unlock(&mutex);

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

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 3);
    }

    return (0);
}

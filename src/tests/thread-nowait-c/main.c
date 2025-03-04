/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <assert.h>
#include <pthread.h>
#include <sched.h>
#include <stdbool.h>

//!
//! This C program tests the creation and joining of threads in a no-std environment using the
//! Nanvix kernel interface. The program consists of a main function that creates a worker thread
//! and waits for it to exit, and a worker function that performs some operations and then exits.
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

    while (true) {
        sched_yield();
    }

    // NOTE: We never get here.

    // Exit the worker thread and make sure it returns the expected value.
    return ((void *)EXPECTED_EXIT_STATUS);
}

/**
 * @brief Tests the creation and joining of threads.
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

    // Don't wait for the worker thread to exit, if something goes wrong this test will hang.

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 3);
    }

    return (0);
}

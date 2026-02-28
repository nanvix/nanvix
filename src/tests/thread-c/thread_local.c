/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>

//==================================================================================================
// Constants
//==================================================================================================

// Expected value for the main thread's thread-local variable.
static const int EXPECTED_MAIN_THREAD_THREAD_LOCAL_VARIABLE_VALUE = 0x1234;

// Expected value for the worker thread's thread-local variable.
static const int EXPECTED_WORKER_THREAD_THREAD_LOCAL_VARIABLE_VALUE = 0xabcd;

//==================================================================================================
// Global Variables
//==================================================================================================

/*
 * A thread-local variable used for testing.
 *
 * Notes:
 *
 * - This variable is declared as `_Thread_local` to ensure that each thread has its own instance.
 * - This variable is declared as non-static and `volatile` to prevent the compiler from optimizing
 *   it away.
 */
_Thread_local volatile int thread_local_var = 0;

/*
 * Zero-initialized thread-local variable for regression testing of overlapping TLS.
 *
 * NOTE: Do NOT add an explicit `= 0` initializer. Without one, the compiler places this variable in
 * .tbss (NOBITS); adding `= 0` would move it to .tdata (PROGBITS), which would mask the overlap bug
 * this test is designed to catch.
 */
_Thread_local volatile int thread_local_zero_init_var;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Worker thread for thread-local storage test.
static void *worker_thread(void *arg)
{
    (void)arg;

    // Verify that zero-initialized thread-local variable starts at zero (GitHub issue #1486).
    assert(thread_local_zero_init_var == 0);

    // Set thread-local variable.
    thread_local_var = EXPECTED_WORKER_THREAD_THREAD_LOCAL_VARIABLE_VALUE;

    // Verify that the thread-local variable was set correctly.
    assert(thread_local_var == EXPECTED_WORKER_THREAD_THREAD_LOCAL_VARIABLE_VALUE);

    // Return the value that was set for verification.
    return ((void *)thread_local_var);
}

// Main thread for thread-local storage test.
static void main_thread(void)
{
    // Verify that zero-initialized thread-local variable starts at zero (GitHub issue #1486).
    assert(thread_local_zero_init_var == 0);

    // Set thread-local variable.
    thread_local_var = EXPECTED_MAIN_THREAD_THREAD_LOCAL_VARIABLE_VALUE;

    // Verify that the main thread's thread-local variable is still unchanged.
    assert(thread_local_var == EXPECTED_MAIN_THREAD_THREAD_LOCAL_VARIABLE_VALUE);

    // Create a worker thread.
    pthread_t worker_tid = PTHREAD_NULL;
    int ret = pthread_create(&worker_tid, NULL, worker_thread, NULL);
    assert(ret == 0);
    assert(worker_tid != PTHREAD_NULL);

    // Wait for the worker thread to exit and get its return value.
    void *retval = NULL;
    ret = pthread_join(worker_tid, &retval);
    assert(ret == 0);
    assert(retval == (void *)EXPECTED_WORKER_THREAD_THREAD_LOCAL_VARIABLE_VALUE);

    // Verify that the main thread's thread-local variable is still unchanged.
    assert(thread_local_var == EXPECTED_MAIN_THREAD_THREAD_LOCAL_VARIABLE_VALUE);
}

/**
 * @brief Tests thread local storage.
 *
 * @returns Nothing. If the test fails, the program will abort.
 */
void test_thread_local(void)
{
    fprintf(stderr, "testing thread local storage ... ");

    main_thread();

    fprintf(stderr, "passed\n");
}

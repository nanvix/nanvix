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
#include <sys/wait.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

// Cleanup event recorded by the main thread.
#define MAIN_THREAD_EVENT 1

// First cleanup event pushed by the worker thread.
#define WORKER_FIRST_EVENT 2

// Second cleanup event pushed by the worker thread.
#define WORKER_SECOND_EVENT 3

// Maximum number of cleanup events.
#define CLEANUP_EVENTS_MAX 3

// Expected worker thread exit status.
#define EXPECTED_EXIT_STATUS ((void *)0xdeadbeef)

//==================================================================================================
// Structures
//==================================================================================================

// Recorded cleanup events.
struct cleanup_state {
    int events[CLEANUP_EVENTS_MAX];
    size_t count;
};

// Argument passed to a cleanup handler.
struct cleanup_arg {
    struct cleanup_state *state;
    int event;
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Records a cleanup event.
static void record_cleanup(void *arg)
{
    struct cleanup_arg *cleanup_arg = (struct cleanup_arg *)arg;
    assert(cleanup_arg->state->count < CLEANUP_EVENTS_MAX);
    cleanup_arg->state->events[cleanup_arg->state->count++] = cleanup_arg->event;
}

// Returns normally with a pending cleanup handler.
static void *returning_worker(void *arg)
{
    struct cleanup_arg *cleanup_arg = (struct cleanup_arg *)arg;
    _pthread_cleanup_push(record_cleanup, cleanup_arg);

    return (EXPECTED_EXIT_STATUS);
}

// Exercises cleanup handlers in a worker thread.
static void *cleanup_worker(void *arg)
{
    struct cleanup_state *state = (struct cleanup_state *)arg;
    struct cleanup_arg first = {state, WORKER_FIRST_EVENT};
    struct cleanup_arg second = {state, WORKER_SECOND_EVENT};

    _pthread_cleanup_push(record_cleanup, &first);
    _pthread_cleanup_push(record_cleanup, &second);
    pthread_exit(EXPECTED_EXIT_STATUS);

    return (NULL);
}

// Tests if cancellation cleanup handlers can be pushed and popped.
void test_pthread_cleanup(void)
{
    fprintf(stderr, "testing _pthread_cleanup_push() and _pthread_cleanup_pop() ... ");

    // Verify that popping a handler without executing it removes the handler.
    struct cleanup_state pop_state = {{0}, 0};
    struct cleanup_arg pop_arg = {&pop_state, MAIN_THREAD_EVENT};
    _pthread_cleanup_push(record_cleanup, &pop_arg);
    _pthread_cleanup_pop(0);
    assert(pop_state.count == 0);

    // Verify that popping a handler with a non-zero argument executes the handler.
    _pthread_cleanup_push(record_cleanup, &pop_arg);
    _pthread_cleanup_pop(1);
    assert(pop_state.count == 1);
    assert(pop_state.events[0] == MAIN_THREAD_EVENT);

    // Verify that a NULL routine is a balanced no-op.
    _pthread_cleanup_push(NULL, NULL);
    _pthread_cleanup_pop(1);

    // Verify that returning normally does not execute or retain a pending handler.
    struct cleanup_state return_state = {{0}, 0};
    struct cleanup_arg return_arg = {&return_state, MAIN_THREAD_EVENT};
    pthread_t returning_worker_tid = PTHREAD_NULL;
    int ret = pthread_create(&returning_worker_tid, NULL, returning_worker, &return_arg);
    assert(ret == 0);

    void *retval = NULL;
    ret = pthread_join(returning_worker_tid, &retval);
    assert(ret == 0);
    assert(retval == EXPECTED_EXIT_STATUS);
    assert(return_state.count == 0);

    // Keep a handler pending in the main thread while a worker exits with two pending handlers.
    struct cleanup_state exit_state = {{0}, 0};
    struct cleanup_arg main_arg = {&exit_state, MAIN_THREAD_EVENT};
    _pthread_cleanup_push(record_cleanup, &main_arg);

    pthread_t worker = PTHREAD_NULL;
    ret = pthread_create(&worker, NULL, cleanup_worker, &exit_state);
    assert(ret == 0);

    retval = NULL;
    ret = pthread_join(worker, &retval);
    assert(ret == 0);
    assert(retval == EXPECTED_EXIT_STATUS);

    // Worker handlers run in reverse push order without consuming the main thread's handler.
    assert(exit_state.count == 2);
    assert(exit_state.events[0] == WORKER_SECOND_EVENT);
    assert(exit_state.events[1] == WORKER_FIRST_EVENT);
    assert(return_state.count == 0);

    _pthread_cleanup_pop(1);
    assert(exit_state.count == 3);
    assert(exit_state.events[2] == MAIN_THREAD_EVENT);

    // Verify that fork preserves the calling thread's cleanup stack in both processes.
    struct cleanup_state fork_state = {{0}, 0};
    struct cleanup_arg fork_arg = {&fork_state, MAIN_THREAD_EVENT};
    _pthread_cleanup_push(record_cleanup, &fork_arg);

    pid_t child = fork();
    assert(child >= 0);
    if (child == 0) {
        _pthread_cleanup_pop(1);
        _exit(fork_state.count == 1 && fork_state.events[0] == MAIN_THREAD_EVENT ? 0 : 1);
    }

    int status = 0;
    assert(waitpid(child, &status, 0) == child);
    assert(WIFEXITED(status));
    assert(WEXITSTATUS(status) == 0);
    assert(fork_state.count == 0);

    _pthread_cleanup_pop(1);
    assert(fork_state.count == 1);
    assert(fork_state.events[0] == MAIN_THREAD_EVENT);

    fprintf(stderr, "passed\n");
}

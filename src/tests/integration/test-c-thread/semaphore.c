/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <assert.h>
#include <errno.h>
#include <limits.h>
#include <pthread.h>
#include <sched.h>
#include <semaphore.h>
#include <stddef.h>
#include <stdio.h>

//==================================================================================================
// Constants
//==================================================================================================

// Number of items exchanged in the producer/consumer test.
#define NITERATIONS 32

// Number of scheduler yields granted to a blocked worker to prove it stays blocked.
#define NYIELDS 1000

//==================================================================================================
// Global Variables
//==================================================================================================

// Semaphore that counts filled slots (items ready to be consumed).
static sem_t sem_filled;

// Semaphore that counts empty slots (space available to produce).
static sem_t sem_empty;

// Single-slot buffer shared between the producer and the consumer.
static volatile int shared_slot;

// Semaphore used to check that sem_wait() blocks until a matching sem_post().
static sem_t sem_handoff;

// Flag set by the worker thread just before it blocks on the semaphore.
static volatile int handoff_waiting;

// Flag set by the worker thread once it has been released by the main thread.
static volatile int handoff_done;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests basic counting semantics of a semaphore without blocking.
static void test_sem_basic(void)
{
    fprintf(stderr, "testing sem_init/sem_wait/sem_post/sem_destroy (basic) ... ");

    sem_t sem;

    // Initialize the semaphore with a value of two.
    assert(sem_init(&sem, 0, 2) == 0);

    // Two waits should succeed immediately without blocking.
    assert(sem_wait(&sem) == 0);
    assert(sem_wait(&sem) == 0);

    // Replenish the semaphore and drain it again.
    assert(sem_post(&sem) == 0);
    assert(sem_post(&sem) == 0);
    assert(sem_wait(&sem) == 0);
    assert(sem_wait(&sem) == 0);

    // Destroy the semaphore.
    assert(sem_destroy(&sem) == 0);

    fprintf(stderr, "passed\n");
}

// Worker thread that blocks on `sem_handoff` and records when it is released.
static void *handoff_thread(void *arg)
{
    (void)arg;

    // Announce that we are about to block on the semaphore.
    handoff_waiting = 1;

    // Block until the main thread posts the semaphore.
    assert(sem_wait(&sem_handoff) == 0);
    handoff_done = 1;

    return (NULL);
}

// Tests that sem_wait() blocks until a matching sem_post().
static void test_sem_blocking(void)
{
    fprintf(stderr, "testing sem_wait() blocking ... ");

    handoff_waiting = 0;
    handoff_done = 0;

    // Initialize the semaphore with a value of zero so that a waiter blocks.
    assert(sem_init(&sem_handoff, 0, 0) == 0);

    // Spawn a worker that blocks on the semaphore.
    pthread_t tid = PTHREAD_NULL;
    assert(pthread_create(&tid, NULL, handoff_thread, NULL) == 0);

    // Wait until the worker has reached the point where it blocks on the semaphore.
    while (!handoff_waiting) {
        sched_yield();
    }

    // Give the worker ample opportunity to run: while the semaphore has not been
    // posted, sem_wait() must keep the worker blocked and it must not complete.
    for (int i = 0; i < NYIELDS; i++) {
        assert(handoff_done == 0);
        sched_yield();
    }

    // Release the worker and wait for it to finish.
    assert(sem_post(&sem_handoff) == 0);
    assert(pthread_join(tid, NULL) == 0);

    // The worker must have observed the release.
    assert(handoff_done == 1);

    assert(sem_destroy(&sem_handoff) == 0);

    fprintf(stderr, "passed\n");
}

// Consumer thread of the producer/consumer test.
static void *consumer_thread(void *arg)
{
    (void)arg;

    for (int i = 0; i < NITERATIONS; i++) {
        // Wait for the producer to fill the slot.
        assert(sem_wait(&sem_filled) == 0);

        // Consume the item and check that it matches the expected value.
        assert(shared_slot == i);

        // Signal that the slot is empty again.
        assert(sem_post(&sem_empty) == 0);
    }

    return (NULL);
}

// Tests producer/consumer synchronization using a pair of semaphores.
static void test_sem_producer_consumer(void)
{
    fprintf(stderr, "testing sem producer/consumer ... ");

    // Initially the buffer is empty: no filled slots, one empty slot.
    assert(sem_init(&sem_filled, 0, 0) == 0);
    assert(sem_init(&sem_empty, 0, 1) == 0);
    shared_slot = -1;

    // Spawn the consumer thread.
    pthread_t tid = PTHREAD_NULL;
    assert(pthread_create(&tid, NULL, consumer_thread, NULL) == 0);

    // Produce items one at a time.
    for (int i = 0; i < NITERATIONS; i++) {
        // Wait for an empty slot.
        assert(sem_wait(&sem_empty) == 0);

        // Produce the item.
        shared_slot = i;

        // Signal that the slot is filled.
        assert(sem_post(&sem_filled) == 0);
    }

    // Wait for the consumer to finish.
    assert(pthread_join(tid, NULL) == 0);

    assert(sem_destroy(&sem_filled) == 0);
    assert(sem_destroy(&sem_empty) == 0);

    fprintf(stderr, "passed\n");
}

// Tests that semaphore functions reject invalid arguments.
static void test_sem_einval(void)
{
    fprintf(stderr, "testing sem invalid arguments ... ");

    sem_t sem;

    // A NULL semaphore pointer must be rejected by every operation.
    errno = 0;
    assert(sem_init(NULL, 0, 0) == -1);
    assert(errno == EINVAL);

    // Process-shared semaphores are not supported.
    errno = 0;
    assert(sem_init(&sem, 1, 0) == -1);
    assert(errno == ENOTSUP);

    errno = 0;
    assert(sem_wait(NULL) == -1);
    assert(errno == EINVAL);

    errno = 0;
    assert(sem_post(NULL) == -1);
    assert(errno == EINVAL);

    errno = 0;
    assert(sem_destroy(NULL) == -1);
    assert(errno == EINVAL);

    // An initial value larger than the maximum must be rejected.
    errno = 0;
    assert(sem_init(&sem, 0, UINT_MAX) == -1);
    assert(errno == EINVAL);

    fprintf(stderr, "passed\n");
}

// Tests unnamed POSIX semaphores.
void test_semaphore(void)
{
    test_sem_basic();
    test_sem_blocking();
    test_sem_producer_consumer();
    test_sem_einval();
}

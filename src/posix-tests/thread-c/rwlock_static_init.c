/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

// Enable rwlock API in toolchain headers.
#define _POSIX_READER_WRITER_LOCKS 1

#include <assert.h>
#include <pthread.h>
#include <sched.h>
#include <stdio.h>

//==================================================================================================
// Global Variables
//==================================================================================================

// Statically initialized read-write lock under test.
static pthread_rwlock_t rwlock = PTHREAD_RWLOCK_INITIALIZER;

// Test state machine:
// 0 -> initial
// 1 -> reader A acquired lock
// 2 -> reader B acquired lock while A still holds (concurrency proven)
// 3 -> reader A released
// 4 -> reader B released
static volatile int stage = 0;
static volatile int concurrency_observed = 0;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Reader thread routine.
static void *reader_a(void *arg)
{
    (void)arg;
    assert(pthread_rwlock_rdlock(&rwlock) == 0);
    stage = 1; // acquired
    // Wait until reader B also acquires (stage moves to 2).
    while (stage < 2) {
        sched_yield();
    }
    assert(pthread_rwlock_unlock(&rwlock) == 0);
    stage = 3;
    return (void *)0;
}

static void *reader_b(void *arg)
{
    (void)arg;
    // Wait until reader A has acquired (stage==1).
    while (stage < 1) {
        sched_yield();
    }
    assert(pthread_rwlock_rdlock(&rwlock) == 0);
    if (stage == 1) {
        // A still holds lock, so concurrency achieved.
        concurrency_observed = 1;
    }
    stage = 2; // mark that B acquired
    // Wait until A releases (stage>=3) before B releases so ordering predictable.
    while (stage < 3) {
        sched_yield();
    }
    assert(pthread_rwlock_unlock(&rwlock) == 0);
    stage = 4;
    return (void *)0;
}

// Tests if statically initialized read-write locks support multiple concurrent readers.
void test_pthread_rwlock_static_init(void)
{
    fprintf(stderr, "testing pthread_rwlock_static_init() ... ");

    pthread_t a = 0, b = 0;
    assert(pthread_create(&a, NULL, reader_a, NULL) == 0);
    assert(pthread_create(&b, NULL, reader_b, NULL) == 0);

    void *rv = (void *)1;
    assert(pthread_join(a, &rv) == 0 && rv == 0);
    assert(pthread_join(b, &rv) == 0 && rv == 0);

    assert(stage == 4);
    assert(concurrency_observed == 1);

    fprintf(stderr, "passed\n");
}

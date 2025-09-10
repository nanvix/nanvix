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

// Flags for synchronization validation.
static volatile int writer_inside = 0;
static volatile int reader_inside = 0;
static volatile int reader_started_after_writer = 0;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Writer thread: acquires write lock, holds briefly, then releases.
static void *writer_thread(void *arg)
{
    pthread_rwlock_t *rwlock = (pthread_rwlock_t *)arg;

    assert(pthread_rwlock_wrlock(rwlock) == 0);
    writer_inside = 1;
    for (int i = 0; i < 4000; i++) {
        sched_yield();
    }
    writer_inside = 0;
    assert(pthread_rwlock_unlock(rwlock) == 0);
    return (void *)0;
}

// Reader thread: should observe writer has left before entering.
static void *reader_thread(void *arg)
{
    pthread_rwlock_t *rwlock = (pthread_rwlock_t *)arg;
    assert(pthread_rwlock_rdlock(rwlock) == 0);
    // By contract of scheduling in the test, writer should no longer be inside.
    if (!writer_inside) {
        reader_started_after_writer = 1;
    }
    reader_inside = 1;
    for (int i = 0; i < 2000; i++) {
        sched_yield();
    }
    reader_inside = 0;
    assert(pthread_rwlock_unlock(rwlock) == 0);
    return (void *)0;
}

// Tests dynamic initialization, lock exclusion between writer and reader, and destroy semantics.
void test_pthread_rwlock_dynamic_init(void)
{
    fprintf(stderr, "testing pthread_rwlock_dynamic_init() ... ");

    pthread_rwlock_t rwlock = 0; // Uninitialized object.
    assert(pthread_rwlock_init(&rwlock, NULL) == 0);

    // Test that destroying while locked fails.
    assert(pthread_rwlock_wrlock(&rwlock) == 0);
    int destroy_ret = pthread_rwlock_destroy(&rwlock);
    assert(destroy_ret != 0); // Should fail because lock is busy.
    assert(pthread_rwlock_unlock(&rwlock) == 0);

    // Launch writer first so that reader blocks until writer releases.
    pthread_t writer_tid = 0;
    assert(pthread_create(&writer_tid, NULL, writer_thread, (void *)&rwlock) == 0);

    // Wait until writer holds the lock.
    while (!writer_inside) {
        sched_yield();
    }

    // Launch reader which must block until writer releases the lock.
    pthread_t reader_tid = 0;
    assert(pthread_create(&reader_tid, NULL, reader_thread, (void *)&rwlock) == 0);

    // While writer is inside, reader must not have entered.
    while (writer_inside) {
        assert(reader_inside == 0);
        sched_yield();
    }

    // Join threads.
    void *retval = (void *)1;
    assert(pthread_join(writer_tid, &retval) == 0);
    assert(retval == 0);
    assert(pthread_join(reader_tid, &retval) == 0);
    assert(retval == 0);

    // Reader must have started only after writer exited.
    assert(reader_started_after_writer == 1);

    // Now destroy must succeed.
    assert(pthread_rwlock_destroy(&rwlock) == 0);

    fprintf(stderr, "passed\n");
}

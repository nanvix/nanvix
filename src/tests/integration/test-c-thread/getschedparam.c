/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <errno.h>
#include <limits.h>
#include <pthread.h>
#include <sched.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Asserts that a thread runs under the default scheduling policy and priority.
static void assert_default_schedparam(pthread_t thread)
{
    int policy = -1;
    struct sched_param param = {
        .sched_priority = -1,
    };
    int ret = pthread_getschedparam(thread, &policy, &param);
    assert(ret == 0);
    assert(policy == SCHED_OTHER);
    assert(param.sched_priority == 0);
}

// Worker thread.
static void *worker_thread(void *arg)
{
    const pthread_t main_thread = *(const pthread_t *)arg;

    // A thread can retrieve its own scheduling policy and parameters.
    assert_default_schedparam(pthread_self());

    // A thread can retrieve the scheduling policy and parameters of another thread.
    assert_default_schedparam(main_thread);

    return NULL;
}

// Tests if the scheduling policy and parameters of a thread can be retrieved.
void test_pthread_getschedparam(void)
{
    fprintf(stderr, "testing pthread_getschedparam() ... ");

    // The main thread can retrieve its own scheduling policy and parameters.
    pthread_t main_thread = pthread_self();
    assert_default_schedparam(main_thread);

    // The scheduling policy and parameters must match the attributes of the thread.
    pthread_attr_t attr = {
        0,
    };
    int ret = pthread_getattr_np(main_thread, &attr);
    assert(ret == 0);
    struct sched_param attr_param = {
        .sched_priority = -1,
    };
    ret = pthread_attr_getschedparam(&attr, &attr_param);
    assert(ret == 0);
    int policy = -1;
    struct sched_param param = {
        .sched_priority = -1,
    };
    ret = pthread_getschedparam(main_thread, &policy, &param);
    assert(ret == 0);
    assert(policy == attr.schedpolicy);
    assert(param.sched_priority == attr_param.sched_priority);
    ret = pthread_attr_destroy(&attr);
    assert(ret == 0);

    // The scheduling policy and parameters of a worker thread can be retrieved by other threads,
    // and the worker thread can retrieve its own and those of other threads.
    pthread_t worker_tid = PTHREAD_NULL;
    ret = pthread_create(&worker_tid, NULL, worker_thread, &main_thread);
    assert(ret == 0);
    assert_default_schedparam(worker_tid);
    ret = pthread_join(worker_tid, NULL);
    assert(ret == 0);

    // Thread identifiers that cannot identify a thread must be rejected without modifying the
    // output arguments.
    const pthread_t invalid_threads[] = {
        PTHREAD_NULL,
        (pthread_t)INT_MAX + 1,
        (pthread_t)-1,
    };
    for (size_t i = 0; i < sizeof(invalid_threads) / sizeof(invalid_threads[0]); i++) {
        policy = -1;
        param.sched_priority = -1;
        ret = pthread_getschedparam(invalid_threads[i], &policy, &param);
        assert(ret == ESRCH);
        assert(policy == -1);
        assert(param.sched_priority == -1);
    }

    // Invalid pointers must be rejected.
    ret = pthread_getschedparam(main_thread, NULL, &param);
    assert(ret == EINVAL);
    ret = pthread_getschedparam(main_thread, &policy, NULL);
    assert(ret == EINVAL);

    // Misaligned pointers must be rejected. They are built from integers, because converting a
    // pointer to an object pointer that is not correctly aligned is undefined behavior.
    _Alignas(int) unsigned char policy_storage[sizeof(int) + 1];
    int *misaligned_policy = (int *)((uintptr_t)policy_storage + 1);
    ret = pthread_getschedparam(main_thread, misaligned_policy, &param);
    assert(ret == EINVAL);
    _Alignas(struct sched_param) unsigned char param_storage[sizeof(struct sched_param) + 1];
    struct sched_param *misaligned_param = (struct sched_param *)((uintptr_t)param_storage + 1);
    ret = pthread_getschedparam(main_thread, &policy, misaligned_param);
    assert(ret == EINVAL);

    fprintf(stderr, "passed\n");
}

/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_PTHREAD_H
#define _NANVIX_PTHREAD_H

/**
 * @file pthread.h
 * @brief Threads.
 *
 * Declares the POSIX threads constants and attribute types. The scalar thread
 * identifiers are declared in <sys/types.h>; the layouts mirror the Rust
 * definitions in the sysapi crate (pthread.rs, sys_types.rs).
 */

#include <sched.h>
#include <sys/types.h>
#include <time.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#define PTHREAD_MUTEX_NORMAL 0 /**< Mutex that does not detect deadlock. */
#define PTHREAD_MUTEX_RECURSIVE 1 /**< Mutex that allows recursive locking. */
#define PTHREAD_MUTEX_ERRORCHECK 2 /**< Mutex that provides error checking. */
#define PTHREAD_MUTEX_DEFAULT 3 /**< Default mutex type. */
#define PTHREAD_MUTEX_INITIALIZER 0xffffffff /**< Static mutex initializer. */
#define PTHREAD_COND_INITIALIZER 0xffffffff /**< Static condition variable initializer. */
#define PTHREAD_RWLOCK_INITIALIZER 0xffffffff /**< Static read-write lock initializer. */
#define PTHREAD_NULL 0 /**< Null thread identifier. */
#define PTHREAD_CREATE_JOINABLE 0 /**< Thread is created joinable. */
#define PTHREAD_CREATE_DETACHED 1 /**< Thread is created detached. */
#define PTHREAD_CANCEL_ENABLE 0 /**< Thread cancellation is enabled. */
#define PTHREAD_CANCEL_DISABLE 1 /**< Thread cancellation is disabled. */
#define PTHREAD_PROCESS_PRIVATE 0 /**< Synchronization object is private to a process. */
#define PTHREAD_PROCESS_SHARED 1 /**< Synchronization object is shared between processes. */

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Thread attributes. */
typedef struct {
    int is_initialized;            /**< Whether the attributes are initialized. */
    void *stackaddr;               /**< Stack base address.                     */
    size_t stacksize;              /**< Stack size.                             */
    size_t guardsize;              /**< Guard size.                             */
    int contentionscope;           /**< Contention scope.                       */
    int inheritsched;              /**< Inherit-scheduler attribute.            */
    int schedpolicy;               /**< Scheduling policy.                      */
    struct sched_param schedparam; /**< Scheduling parameters.                  */
    int cputime_clock_allowed;     /**< Whether a CPU-time clock is allowed.    */
    int detachstate;               /**< Detach state.                           */
} pthread_attr_t;

/** @brief Condition variable attributes. */
typedef struct {
    int is_initialized; /**< Whether the attributes are initialized. */
    clock_t clock;      /**< Clock used for timeouts.                */
    int pshared;        /**< Process-sharing attribute.              */
} pthread_condattr_t;

/** @brief Mutex attributes. */
typedef struct {
    int is_initialized; /**< Whether the attributes are initialized. */
    int type;           /**< Type of mutex.                          */
    int recursive;      /**< Whether the mutex is recursive.         */
    int pshared;        /**< Process-sharing attribute.              */
} pthread_mutexattr_t;

/** @brief Read-write lock attributes. */
typedef struct {
    int is_initialized; /**< Whether the attributes are initialized. */
} pthread_rwlockattr_t;

/** @brief One-time initialization control. */
typedef struct {
    int is_initialized; /**< Whether the control is initialized. */
    int init_executed;  /**< Whether the initializer has run.    */
} pthread_once_t;

/** @brief Static initializer for a pthread_once_t control variable. */
#define PTHREAD_ONCE_INIT                                                                          \
    {                                                                                              \
        1, 0                                                                                       \
    }

/*==================================================================================================
 * Threads
 *==================================================================================================*/

extern void _pthread_cleanup_pop(int execute);
extern void _pthread_cleanup_push(void (*routine)(void *), void *arg);
extern int pthread_create(pthread_t *thread, const pthread_attr_t *attr, void *(*start_routine)(void *), void *arg);
extern int pthread_join(pthread_t thread, void **retval_ptr);
extern int pthread_detach(pthread_t thread);
extern int pthread_kill(pthread_t thread, int sig);
extern void pthread_exit(void *retval);
extern pthread_t pthread_self(void);
extern int pthread_equal(pthread_t thread1, pthread_t thread2);
extern int pthread_setcancelstate(int state, int *oldstate);
extern void pthread_testcancel(void);

/*==================================================================================================
 * Thread Attributes
 *==================================================================================================*/

extern int pthread_attr_init(pthread_attr_t *attr);
extern int pthread_attr_destroy(pthread_attr_t *attr);
extern int pthread_attr_getdetachstate(const pthread_attr_t *attr, int *detachstate);
extern int pthread_attr_getguardsize(const pthread_attr_t *attr, size_t *guardsize);
extern int pthread_attr_setguardsize(pthread_attr_t *attr, size_t guardsize);
extern int pthread_attr_setdetachstate(pthread_attr_t *attr, int detachstate);
extern int pthread_attr_getschedparam(const pthread_attr_t *attr, struct sched_param *param);
extern int pthread_attr_setschedparam(pthread_attr_t *attr, const struct sched_param *param);
extern int pthread_attr_getstack(const pthread_attr_t *attr, void **stackaddr, size_t *stacksize);
extern int pthread_attr_setstack(pthread_attr_t *attr, void *stackaddr, size_t stacksize);
extern int pthread_attr_getstacksize(const pthread_attr_t *attr, size_t *stacksize);
extern int pthread_attr_setstacksize(pthread_attr_t *attr, size_t stacksize);
extern int pthread_attr_getstackaddr(const pthread_attr_t *attr, void **stackaddr);
extern int pthread_attr_setstackaddr(pthread_attr_t *attr, void *stackaddr);
extern int pthread_getattr_np(pthread_t thread, pthread_attr_t *attr);

/*==================================================================================================
 * Mutexes
 *==================================================================================================*/

extern int pthread_mutex_init(pthread_mutex_t *mutex, const pthread_mutexattr_t *attr);
extern int pthread_mutex_destroy(pthread_mutex_t *mutex);
extern int pthread_mutex_lock(pthread_mutex_t *mutex);
extern int pthread_mutex_unlock(pthread_mutex_t *mutex);
extern int pthread_mutex_trylock(pthread_mutex_t *mutex);
extern int pthread_mutex_timedlock(pthread_mutex_t *mutex, const struct timespec *abstime);
extern int pthread_mutexattr_init(pthread_mutexattr_t *attr);
extern int pthread_mutexattr_destroy(pthread_mutexattr_t *attr);
extern int pthread_mutexattr_getpshared(const pthread_mutexattr_t *attr, int *pshared);
extern int pthread_mutexattr_setpshared(pthread_mutexattr_t *attr, int pshared);
extern int pthread_mutexattr_settype(pthread_mutexattr_t *attr, int type_);
extern int pthread_mutexattr_gettype(const pthread_mutexattr_t *attr, int *type_);

/*==================================================================================================
 * Condition Variables
 *==================================================================================================*/

extern int pthread_cond_init(pthread_cond_t *cond, const pthread_condattr_t *attr);
extern int pthread_cond_destroy(pthread_cond_t *cond);
extern int pthread_cond_signal(const pthread_cond_t *cond);
extern int pthread_cond_broadcast(const pthread_cond_t *cond);
extern int pthread_cond_wait(const pthread_cond_t *cond, pthread_mutex_t *mutex);
extern int pthread_cond_timedwait(const pthread_cond_t *cond, pthread_mutex_t *mutex, const struct timespec *abstime);
extern int pthread_condattr_destroy(pthread_condattr_t *attr);
extern int pthread_condattr_init(pthread_condattr_t *attr);
extern int pthread_condattr_getclock(const pthread_condattr_t *attr, clockid_t *clock_id);
extern int pthread_condattr_getpshared(const pthread_condattr_t *attr, int *pshared);
extern int pthread_condattr_setclock(pthread_condattr_t *attr, clockid_t clock_id);
extern int pthread_condattr_setpshared(pthread_condattr_t *attr, int pshared);

/*==================================================================================================
 * Read-Write Locks
 *==================================================================================================*/

extern int pthread_rwlock_init(pthread_rwlock_t *rwlock, const pthread_rwlockattr_t *attr);
extern int pthread_rwlock_destroy(pthread_rwlock_t *rwlock);
extern int pthread_rwlock_rdlock(pthread_rwlock_t *rwlock);
extern int pthread_rwlock_wrlock(pthread_rwlock_t *rwlock);
extern int pthread_rwlock_unlock(pthread_rwlock_t *rwlock);

/*==================================================================================================
 * Thread-Specific Data
 *==================================================================================================*/

extern int pthread_key_create(pthread_key_t *key_ptr, void (*destructor)(void *));
extern int pthread_key_delete(pthread_key_t key);
extern void *pthread_getspecific(pthread_key_t key);
extern int pthread_setspecific(pthread_key_t key, const void *value);
extern int pthread_once(pthread_once_t *once_control, void (*init_routine)(void));

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_PTHREAD_H */

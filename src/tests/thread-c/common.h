/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _COMMON_H_
#define _COMMON_H_

// Tests if threads can get their own identifiers.
extern void test_pthread_self(void);

// Tests if `pthread_mutex_cond_timedwait()` can be used for synchronization.
extern void test_pthread_cond_timedwait(void);

// Tests if threads can be created and joined.
extern void test_pthread_create_join(void);

// Tests if statically initialized mutexes can be used for synchronization.
extern void test_pthread_mutex_static_init(void);

// Tests if dynamically initialized mutexes can be used for synchronization.
extern void test_pthread_mutex_dynamic_init(void);

// Tests if calling exit() causes the program to exit even if there are other threads running.
extern void test_pthread_nowait(void);

// Tests if statically initialized condition variables can be used for synchronization.
extern void test_pthread_cond_static_init(void);

// Tests if thread interface for operating on thread-specific data works.
extern void test_pthread_tda(void);

// Tests if calling `test_pthread_mutex_timedlock() works.
extern void test_pthread_mutex_timedlock(void);

// Tests if calling `pthread_mutex_trylock() works.
extern void test_pthread_mutex_trylock(void);

// Tests if thread local storage works.
extern void test_thread_local(void);

// Tests if pthread attributes can be initialized and destroyed.
extern void test_pthread_attr_init_destroy(void);

// Tests if pthread_getattr_np() can retrieve attributes and they can be destroyed.
extern void test_pthread_getattr_np_destroy(void);

// Tests if pthread_attr_getstack() can retrieve stack attributes and they can be destroyed.
extern void test_pthread_attr_getstack(void);

// Tests if statically initialized read-write locks can synchronize multiple readers.
extern void test_pthread_rwlock_static_init(void);

// Tests if dynamically initialized read-write locks work (init/destroy + exclusion semantics).
extern void test_pthread_rwlock_dynamic_init(void);

#endif

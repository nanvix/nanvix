/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _COMMON_H_
#define _COMMON_H_

// Tests if threads can get their own identifiers.
extern void test_pthread_self(void);

// Tests if threads can be created and joined.
extern void test_pthread_create_join(void);

// Tests if mutexes can be used for synchronization.
extern void test_pthread_mutex(void);

// Tests if calling exit() causes the program to exit even if there are other threads running.
extern void test_pthread_nowait(void);

#endif

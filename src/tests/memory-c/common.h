/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _COMMON_H_
#define _COMMON_H_

// Tests whether we can allocate memory using `sbrk()`.
extern void test_sbrk(void);

// Tests whether we can map and unmap memory using `mmap()` and `munmap()`.
extern void test_mmap_munmap(void);

// Tests whether we can allocate and free memory using `malloc()` and `free()`.
extern void test_malloc_free(void);

#endif

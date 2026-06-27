/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _COMMON_H_
#define _COMMON_H_

// Tests whether we can map and unmap memory using `mmap()` and `munmap()`.
extern void test_mmap_munmap(void);

// Tests whether we can allocate and free memory using `malloc()` and `free()`.
extern void test_malloc_free(void);

// Tests whether we can allocate and free aligned memory using `aligned_alloc()` and `free()`.
extern void test_aligned_alloc_free(void);

// Tests whether we can query the usable size of allocated blocks using `malloc_usable_size()`.
extern void test_malloc_usable_size(void);

// Stresses the heap allocator with alloc/free cycles under fragmentation pressure.
extern void test_heap_reclaim(void);

// Tests the allocator by attempting to consume the maximum allowed heap capacity.
extern void test_heap_max_capacity(void);

// Exercises the heap shrink path through bulk-free, shrink-to-minimum, anchor-pinned,
// repeated cycles, and small-heap scenarios.
extern void test_heap_shrink(void);

// Tests realloc growth, shrink, NULL-ptr, data preservation, and alignment boundary sizes.
extern void test_realloc(void);

#endif

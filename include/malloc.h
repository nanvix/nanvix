/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_MALLOC_H
#define _NANVIX_MALLOC_H

/**
 * @file malloc.h
 * @brief Memory allocation.
 *
 * Declares the memory-allocation interfaces, including the malloc_usable_size()
 * extension. The prototypes are generated from the libc_stdlib Rust crate.
 */

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Memory Allocation
 *==================================================================================================*/

extern void *malloc(size_t size);
extern void free(void *ptr);
extern void *calloc(size_t nmemb, size_t size);
extern void *realloc(void *ptr, size_t size);
extern void *aligned_alloc(size_t alignment, size_t size);
extern size_t malloc_usable_size(void *ptr);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_MALLOC_H */

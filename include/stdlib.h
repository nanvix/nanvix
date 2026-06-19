/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_STDLIB_H
#define _NANVIX_STDLIB_H

/**
 * @file stdlib.h
 * @brief General utilities.
 *
 * Declares memory allocation and environment interfaces implemented by the
 * libc_stdlib Rust crate.
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
extern int posix_memalign(void **memptr, size_t alignment, size_t size);

/*==================================================================================================
 * Environment
 *==================================================================================================*/

extern char *getenv(const char *name);
extern int setenv(const char *name, const char *value, int overwrite);
extern int unsetenv(const char *name);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_STDLIB_H */

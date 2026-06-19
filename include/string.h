/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_STRING_H
#define _NANVIX_STRING_H

/**
 * @file string.h
 * @brief String and memory operations.
 *
 * Declares byte-string length and memory manipulation routines implemented by
 * the libc_string Rust crate.
 */

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Memory Operations
 *==================================================================================================*/

extern void *memcpy(void *dest, const void *src, size_t len);
extern void *memmove(void *dest, const void *src, size_t len);
extern void *memset(void *ptr, int val, size_t len);
extern int memcmp(const void *ptr1, const void *ptr2, size_t len);

/*==================================================================================================
 * String Operations
 *==================================================================================================*/

extern size_t strlen(const char *s);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_STRING_H */

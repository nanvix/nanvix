/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_STRINGS_H
#define _NANVIX_STRINGS_H

/**
 * @file strings.h
 * @brief Legacy BSD string operations.
 */

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

extern int strcasecmp(const char *s1, const char *s2);
extern int strncasecmp(const char *s1, const char *s2, size_t n);
extern int bcmp(const void *s1, const void *s2, size_t n);
extern void bcopy(const void *src, void *dest, size_t n);
extern void bzero(void *s, size_t n);
extern int ffs(int i);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_STRINGS_H */

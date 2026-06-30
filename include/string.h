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
#include <locale.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Memory Operations
 *==================================================================================================*/

extern void *memccpy(void *dest, const void *src, int c, size_t n);
extern void *memchr(const void *s, int c, size_t n);
extern int memcmp(const void *ptr1, const void *ptr2, size_t len);
extern void *memcpy(void *dest, const void *src, size_t len);
extern void *memmove(void *dest, const void *src, size_t len);
extern void *mempcpy(void *dest, const void *src, size_t len);
extern void *memrchr(const void *s, int c, size_t n);
extern void *memset(void *ptr, int val, size_t len);

/*==================================================================================================
 * String Operations
 *==================================================================================================*/

extern char *stpcpy(char *dest, const char *src);
extern char *stpncpy(char *dest, const char *src, size_t n);
extern int strcasecmp(const char *s1, const char *s2);
extern char *strcasestr(const char *haystack, const char *needle);
extern char *strcat(char *dest, const char *src);
extern char *strchr(const char *s, int c);
extern char *strchrnul(const char *s, int c);
extern int strcmp(const char *s1, const char *s2);
extern int strcoll(const char *s1, const char *s2);
extern char *strcpy(char *dest, const char *src);
extern size_t strcspn(const char *s, const char *reject);
extern char *strdup(const char *s);
extern char *strerror(int errnum);
extern int strerror_r(int errnum, char *buf, size_t buflen);
extern size_t strlcat(char *dest, const char *src, size_t size);
extern size_t strlcpy(char *dest, const char *src, size_t size);
extern size_t strlen(const char *s);
extern int strncasecmp(const char *s1, const char *s2, size_t n);
extern char *strncat(char *dest, const char *src, size_t n);
extern int strncmp(const char *s1, const char *s2, size_t n);
extern char *strncpy(char *dest, const char *src, size_t n);
extern char *strndup(const char *s, size_t n);
extern size_t strnlen(const char *s, size_t maxlen);
extern char *strpbrk(const char *s, const char *accept);
extern char *strrchr(const char *s, int c);
extern char *strsep(char **stringp, const char *delim);
extern char *strsignal(int sig);
extern size_t strspn(const char *s, const char *accept);
extern char *strstr(const char *haystack, const char *needle);
extern char *strtok(char *s, const char *delim);
extern char *strtok_r(char *s, const char *delim, char **saveptr);
extern int strverscmp(const char *s1, const char *s2);
extern size_t strxfrm(char *dest, const char *src, size_t n);
extern int strcoll_l(const char *s1, const char *s2, locale_t locale);
extern size_t strxfrm_l(char *dest, const char *src, size_t n, locale_t locale);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_STRING_H */

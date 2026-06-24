/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_WCHAR_H
#define _NANVIX_WCHAR_H

/**
 * @file wchar.h
 * @brief Wide character strings.
 *
 * Declares functions for wide character string manipulation and memory
 * operations. Implemented by the libc_wchar Rust crate.
 */

#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <locale.h>
#include <time.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

#ifndef _WCHAR_T_DEFINED
#define _WCHAR_T_DEFINED
#ifndef __cplusplus
typedef __WCHAR_TYPE__ wchar_t;
#endif
#endif

#ifndef _WINT_T_DEFINED
#define _WINT_T_DEFINED
typedef int wint_t;
#endif

#ifndef _MBSTATE_T_DEFINED
#define _MBSTATE_T_DEFINED
/** @brief Conversion state for the restartable multibyte functions. */
typedef struct {
    int __count;       /**< Number of pending bytes.            */
    unsigned char __buf[4]; /**< Buffered bytes of an incomplete seq. */
} mbstate_t;
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#ifndef WEOF
#define WEOF ((wint_t)(-1))
#endif

#ifndef WCHAR_MAX
#define WCHAR_MAX 0x7fffffff
#endif

#ifndef WCHAR_MIN
#define WCHAR_MIN (-WCHAR_MAX - 1)
#endif

#ifndef NULL
#define NULL ((void *)0)
#endif

/*==================================================================================================
 * String Operations
 *==================================================================================================*/

extern size_t wcslen(const wchar_t *s);
extern wchar_t *wcscpy(wchar_t *dest, const wchar_t *src);
extern wchar_t *wcsncpy(wchar_t *dest, const wchar_t *src, size_t n);
extern wchar_t *wcscat(wchar_t *dest, const wchar_t *src);

/*==================================================================================================
 * Comparison
 *==================================================================================================*/

extern int wcscmp(const wchar_t *s1, const wchar_t *s2);
extern int wcsncmp(const wchar_t *s1, const wchar_t *s2, size_t n);
extern int wcscoll(const wchar_t *s1, const wchar_t *s2);
extern size_t wcsxfrm(wchar_t *dest, const wchar_t *src, size_t n);

/*==================================================================================================
 * Search
 *==================================================================================================*/

extern wchar_t *wcschr(const wchar_t *s, wchar_t c);
extern wchar_t *wcsrchr(const wchar_t *s, wchar_t c);
extern wchar_t *wcsstr(const wchar_t *haystack, const wchar_t *needle);
extern wchar_t *wcstok(wchar_t *ws, const wchar_t *delim, wchar_t **ptr);

/*==================================================================================================
 * Numeric Conversions
 *==================================================================================================*/

extern long wcstol(const wchar_t *nptr, wchar_t **endptr, int base);
extern unsigned long wcstoul(const wchar_t *nptr, wchar_t **endptr, int base);
extern long long wcstoll(const wchar_t *nptr, wchar_t **endptr, int base);
extern unsigned long long wcstoull(const wchar_t *nptr, wchar_t **endptr, int base);
extern double wcstod(const wchar_t *nptr, wchar_t **endptr);

/*==================================================================================================
 * Wide Memory Operations
 *==================================================================================================*/

extern wchar_t *wmemcpy(wchar_t *dest, const wchar_t *src, size_t n);
extern wchar_t *wmemmove(wchar_t *dest, const wchar_t *src, size_t n);
extern wchar_t *wmemset(wchar_t *s, wchar_t c, size_t n);
extern int wmemcmp(const wchar_t *s1, const wchar_t *s2, size_t n);
extern wchar_t *wmemchr(const wchar_t *s, wchar_t c, size_t n);

/*==================================================================================================
 * Byte / Wide Conversion
 *==================================================================================================*/

extern wint_t btowc(int c);
extern int wctob(wint_t c);

/*==================================================================================================
 * Wide Character Formatted Output
 *==================================================================================================*/

extern int swprintf(wchar_t *ws, size_t n, const wchar_t *format, ...);
extern int vswprintf(wchar_t *ws, size_t n, const wchar_t *format, va_list ap);

/*==================================================================================================
 * Restartable Multibyte / Wide Conversion
 *==================================================================================================*/

extern size_t mbrtowc(wchar_t *pwc, const char *s, size_t n, mbstate_t *ps);
extern size_t wcrtomb(char *s, wchar_t wc, mbstate_t *ps);
extern size_t mbrlen(const char *s, size_t n, mbstate_t *ps);
extern int mbsinit(const mbstate_t *ps);
extern size_t mbsrtowcs(wchar_t *dest, const char **src, size_t n, mbstate_t *ps);
extern size_t wcsrtombs(char *dest, const wchar_t **src, size_t n, mbstate_t *ps);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_WCHAR_H */

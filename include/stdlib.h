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
 * Declares memory allocation, string conversion, integer arithmetic, searching
 * and sorting, pseudo-random sequence generation, environment, and process
 * control interfaces implemented by the libc_stdlib Rust crate.
 */

#include <stddef.h>
#include <sys/wait.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#ifndef EXIT_SUCCESS
#define EXIT_SUCCESS 0 /**< Successful termination for exit(). */
#endif

#ifndef EXIT_FAILURE
#define EXIT_FAILURE 1 /**< Unsuccessful termination for exit(). */
#endif

#ifndef RAND_MAX
#define RAND_MAX 2147483647 /**< Maximum value returned by rand(). */
#endif

#ifndef MB_CUR_MAX
#define MB_CUR_MAX ((size_t)1) /**< Maximum number of bytes in a character for the current locale. */
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

/* glibc compatibility: expose alloca() via <stdlib.h> under _GNU_SOURCE. */
#ifdef _GNU_SOURCE
#include <alloca.h>
#endif

/** @brief Structure type returned by the div() function. */
typedef struct {
    int quot; /**< Quotient.  */
    int rem;  /**< Remainder. */
} div_t;

/** @brief Structure type returned by the ldiv() function. */
typedef struct {
    long quot; /**< Quotient.  */
    long rem;  /**< Remainder. */
} ldiv_t;

/** @brief Structure type returned by the lldiv() function. */
typedef struct {
    long long quot; /**< Quotient.  */
    long long rem;  /**< Remainder. */
} lldiv_t;

/*==================================================================================================
 * Memory Allocation
 *==================================================================================================*/

extern void *malloc(size_t size);
extern void free(void *ptr);
extern void *calloc(size_t nmemb, size_t size);
extern void *realloc(void *ptr, size_t size);
extern void *reallocarray(void *ptr, size_t nelem, size_t elsize);
extern void *aligned_alloc(size_t alignment, size_t size);
extern int posix_memalign(void **memptr, size_t alignment, size_t size);

/*==================================================================================================
 * String Conversion
 *==================================================================================================*/

extern double atof(const char *nptr);
extern int atoi(const char *nptr);
extern long atol(const char *nptr);
extern long long atoll(const char *nptr);
extern double strtod(const char *restrict nptr, char **restrict endptr);
extern float strtof(const char *restrict nptr, char **restrict endptr);
extern long double strtold(const char *restrict nptr, char **restrict endptr);
extern long strtol(const char *restrict nptr, char **restrict endptr, int base);
extern long long strtoll(const char *restrict nptr, char **restrict endptr, int base);
extern unsigned long strtoul(const char *restrict nptr, char **restrict endptr, int base);
extern unsigned long long strtoull(const char *restrict nptr, char **restrict endptr, int base);

/*==================================================================================================
 * Integer Arithmetic
 *==================================================================================================*/

extern int abs(int j);
extern long labs(long j);
extern long long llabs(long long j);
extern div_t div(int numer, int denom);
extern ldiv_t ldiv(long numer, long denom);
extern lldiv_t lldiv(long long numer, long long denom);

/*==================================================================================================
 * Searching and Sorting
 *==================================================================================================*/

extern void *bsearch(const void *key, const void *base, size_t nmemb, size_t size, int (*compar)(const void *, const void *));
extern void qsort(void *base, size_t nmemb, size_t size, int (*compar)(const void *, const void *));
extern void qsort_r(void *base, size_t nmemb, size_t size, int (*compar)(const void *, const void *, void *), void *arg);

/*==================================================================================================
 * Pseudo-Random Sequence Generation
 *==================================================================================================*/

extern int rand(void);
extern void srand(unsigned int seed);

/*==================================================================================================
 * Environment
 *==================================================================================================*/

extern char *getenv(const char *name);
extern char *secure_getenv(const char *name);
extern int setenv(const char *name, const char *value, int overwrite);
extern int unsetenv(const char *name);
extern int putenv(char *string);
extern int clearenv(void);

/*==================================================================================================
 * Process Control
 *==================================================================================================*/

extern _Noreturn void _Exit(int status);
extern _Noreturn void abort(void);
extern int at_quick_exit(void (*func)(void));
extern int atexit(void (*func)(void));
extern _Noreturn void exit(int status);
extern _Noreturn void quick_exit(int status);
extern int system(const char *command);

/*==================================================================================================
 * Multibyte Conversion
 *==================================================================================================*/

extern int mblen(const char *s, size_t n);
extern int mbtowc(wchar_t *restrict pwc, const char *restrict s, size_t n);
extern size_t mbstowcs(wchar_t *restrict dst, const char *restrict src, size_t n);
extern int wctomb(char *s, wchar_t wc);
extern size_t wcstombs(char *restrict dst, const wchar_t *restrict src, size_t n);

/*==================================================================================================
 * Temporary Files
 *==================================================================================================*/

extern char *mkdtemp(char *template);
extern int mkostemp(char *template, int flag);
extern int mkstemp(char *template);
extern char *mktemp(char *template);

/*==================================================================================================
 * Path Utilities
 *==================================================================================================*/

extern char *realpath(const char *restrict path, char *restrict resolved_path);

/*==================================================================================================
 * Pseudo-Terminals
 *==================================================================================================*/

extern int posix_openpt(int flags);
extern int grantpt(int fd);
extern int unlockpt(int fd);
extern char *ptsname(int fd);
extern int ptsname_r(int fd, char *buf, size_t buflen);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_STDLIB_H */

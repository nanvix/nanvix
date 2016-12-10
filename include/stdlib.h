/*
 * Copyright(C) 2016 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * This file is part of Nanvix.
 * 
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 * 
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

/**
 * @file
 * 
 * @brief Standard library definitions.
 */

#ifndef STDLIB_H_
#define STDLIB_H_

	#include <machine/ieeefp.h>
	#include "_ansi.h"

	#define __need_size_t
	#define __need_wchar_t
	#define __need_NULL
	#include <stddef.h>

	#include <sys/reent.h>
	#include <sys/cdefs.h>

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

 	#include <limits.h>
	#include <math.h>
	#include <sys/wait.h>

#endif

 	/**
	 * @name Exit codes.
	 */
	/**@{*/
	#define EXIT_FAILURE 1 /**< Unsuccessful termination for exit(). */
	#define EXIT_SUCCESS 0 /**< Successful termination for exit().   */
	/**@}*/

	/**
	 * @brief Maximum value returned by rand().
	 */
	#define RAND_MAX __RAND_MAX

	/**
	 * @brief Maximum number of bytes in a locale character.
	 */
	#define MB_CUR_MAX 1

 	/**
	 * @brief Structure type returned by the div() function.
	 */
	typedef struct 
	{
	  int quot; /**< Quotient. */
	  int rem; /**<  Remainder. */
	} div_t;

	/**
	 * @brief Structure type returned by the ldiv() function.
	 */
	typedef struct 
	{
	  long quot; /**< Quotient. */
	  long rem; /**<  Remainder. */
	} ldiv_t;

	/**
	 * @brief Structure type returned by the lldiv() function.
	 */
	typedef struct
	{
	  long long int quot; /**< Quotient. */
	  long long int rem; /**<  Remainder. */
	} lldiv_t;


	/**
	 * @defgroup stdlib Standard Library
	 * 
	 * @brief Standard library definitions.
	 */
	/**@{*/

	/* Forward declarations. */
	extern void _Exit(int);
	extern void abort(void);
	extern int abs(int);
	extern int atexit(void (*)(void));
	extern double atof(const char *);
	extern int atoi(const char *);
	extern long atol(const char *);
	extern long long atoll(const char *);
	extern void *bsearch(const void *, const void *, size_t, size_t, 
		int (*)(const void *, const void *));
	extern void *calloc(size_t, size_t);
	extern div_t div(int, int);
	extern void exit(int);
	extern void free(void *);
	extern char *getenv(const char *);
	extern long labs(long);
	extern ldiv_t ldiv(long, long);
	extern long long llabs(long long);
	extern lldiv_t lldiv(long long, long long);
	extern void *malloc(size_t);
	extern int mblen(const char *, size_t);
	extern size_t mbstowcs(wchar_t *restrict, const char *restrict, size_t);
	extern int mbtowc(wchar_t *restrict, const char *restrict, size_t);
	extern void qsort(void *, size_t, size_t, 
		int (*)(const void *, const void *));
	extern int rand(void);
	extern void *realloc(void *, size_t);
	extern void srand(unsigned);
	extern double strtod(const char *restrict, char **restrict);
	extern float strtof(const char *restrict, char **restrict);
	extern long strtol(const char *restrict, char **restrict, int);
	extern long double strtold(const char *restrict, char **restrict);
	extern long long strtoll(const char *restrict, char **restrict, int);
	extern unsigned long strtoul(const char *restrict, char **restrict, int);
	extern unsigned long long strtoull(const char *restrict, char **restrict, int);
	extern int system(const char *);
	extern size_t wcstombs(char *restrict, const wchar_t *restrict, size_t);
	extern int wctomb(char *, wchar_t);

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)
	extern int getsubopt(char **, char *const *, char **);
	extern char *mkdtemp(char *);
	extern int setenv(const char *, const char *, int);
	extern int unsetenv(const char *);
	extern int rand_r(unsigned *);
#endif

#ifdef _XOPEN_SOURCE
	extern long a64l(const char *);
	extern double drand48(void);
	extern double erand48(unsigned short [3]);
	extern long jrand48(unsigned short [3]);
	extern char *l64a(long);
	extern void lcong48(unsigned short [7]);
	extern long lrand48(void);
	extern long mrand48(void);
	extern long nrand48(unsigned short [3]);
	extern int putenv(char *);
	extern unsigned short *seed48(unsigned short [3]);
	extern void srand48(long);
#endif

	/* Newlib needs to those non-standard functions. */
	extern char *_dtoa_r(struct _reent *, double, int, int, int *, int *, char **);
	extern char *_findenv_r(struct _reent *, const char *, int *);
	extern char *_getenv_r(struct _reent *, const char *);
	extern double _strtod_r(struct _reent *, const char *restrict, 
		char **restrict);
	extern int mkstemp(char *);
	extern int _setenv_r(struct _reent *, const char *, const char *, int);
	extern int _unsetenv_r(struct _reent *, const char *);
	extern long _strtol_r(struct _reent *,const char *restrict, char **restrict,
		int);
	extern long long _strtoll_r(struct _reent *, const char *restrict,
		char **restrict, int);
	extern unsigned long _strtoul_r(struct _reent *,const char *restrict,
		char **restrict, int);
	extern unsigned long long _strtoull_r(struct _reent *, const char *restrict,
		char **restrict, int);
	extern void	_free_r(struct _reent *, void *);
	extern void *_calloc_r(struct _reent *, size_t, size_t);
	extern void *_malloc_r(struct _reent *, size_t);
	extern void *_realloc_r(struct _reent *, void *, size_t);

	/**@}*/

#endif /* STDLIB_H_ */

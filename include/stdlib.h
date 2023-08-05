/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#if defined(_POSIX_C_SOURCE)

	#include <stddef.h>
	#include <limits.h>
	#include <math.h>
	#include <sys/wait.h>

#endif

	/**
	 * @defgroup stdlib Standard Library
	 *
	 * @brief Standard library definitions.
	 *
	 * @todo abort()
	 * @todo atof()
	 * @todo mbstowcs()
	 * @todo strtod()
	 * @todo strtold()
	 * @todo strtof()
	 * @todo strtold()
	 * @todo wcstombs()
	 * @todo mkdtemp()
	 * @todo mkstemp()
	 * @todo posix_memalign()
	 * @todo rand_r()
	 * @todo a64l()
	 * @todo drand48()
	 * @todo erand48()
	 * @todo grantpt()
	 * @todo initstate()
	 * @todo jrand48()
	 * @todo l64a()
	 * @todo lcong48()
	 * @todo lrand48()
	 * @todo mrand48()
	 * @todo nrand48()
	 * @todo posix_openpt()
	 * @todo ptsname()
	 * @todo putenv()
	 * @todo random()
	 * @todo realpath()
	 * @todo seed48()
	 * @todo setkey()
	 * @todo setstate()
	 * @todo srand48()
	 * @todo srandom()
	 * @todo unlockpt()
	 */
	/**@{*/

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
	#define RAND_MAX 0x7fffffff

	/**
	 * @brief Maximum number of bytes in a locale character.
	 */
	#define MB_CUR_MAX ((size_t) 1)

	#define _NEED_NULL
	#include <decl.h>

	/**
	 * @brief Structure type returned by the div() function.
	 */
	typedef struct
	{
		int quot; /**< Quotient.  */
		int rem;  /**< Remainder. */
	} div_t;

	/**
	 * @brief Structure type returned by the ldiv() function.
	 */
	typedef struct
	{
		int quot; /**< Quotient.  */
		int rem;  /**< Remainder. */
	} ldiv_t;

	/**
	 * @brief Structure type returned by the lldiv() function.
	 */
	typedef struct
	{
		int quot; /**< Quotient.  */
		int rem;  /**< Remainder. */
	} lldiv_t;

	#define _NEED_SIZE_T
	#define _NEED_WCHAR_T
	#define _NEED_WSTATUS
	#include <decl.h>

	/* Forward definitions. */
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
	extern void qsort(void *, size_t, size_t, int (*)(const void *,
					  const void *));
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

	/* Forward definitions. */
	extern int getsubopt(char **, char *const *, char **);
	extern char *mkdtemp(char *);
	extern int mkstemp(char *);
	extern int posix_memalign(void **, size_t, size_t);
	extern int rand_r(unsigned *);
	extern int setenv(const char *, const char *, int);
	extern int unsetenv(const char *);

#endif

#if defined(_XOPEN_SOURCE)

	/* Forward definitions. */
	extern long a64l(const char *);
	extern double drand48(void);
	extern double erand48(unsigned short [3]);
	extern int grantpt(int);
	extern char *initstate(unsigned, char *, size_t);
	extern long jrand48(unsigned short [3]);
	extern char *l64a(long);
	extern void lcong48(unsigned short [7]);
	extern long lrand48(void);
	extern long mrand48(void);
	extern long nrand48(unsigned short [3]);
	extern int posix_openpt(int);
	extern char *ptsname(int);
	extern int putenv(char *);
	extern long random(void);
	extern char *realpath(const char *restrict, char *restrict);
	extern unsigned short *seed48(unsigned short [3]);
	extern void setkey(const char *);
	extern char *setstate(char *);
	extern void srand48(long);
	extern void srandom(unsigned);
	extern int unlockpt(int);

#endif

	/**@}*/

#endif /* STDLIB_H_ */

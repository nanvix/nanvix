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
 * @brief Wide-character handler library.
 */

#ifndef WCHAR_H_
#define WCHAR_H_

	#include <_ansi.h>
	#include <sys/reent.h>

	#define __need_size_t
	#define __need_wchar_t
	#define __need_wint_t
	#define __need_NULL
	#include <stddef.h>

	#define __need___va_list
	#include <stdarg.h>

	#include <sys/_types.h>
	#include <sys/cdefs.h>
	#include <sys/features.h>

#ifndef WEOF
	#define WEOF ((wint_t)-1)
#endif

	/* WCHAR_MIN definition. */
#ifndef WCHAR_MIN
	#ifdef __WCHAR_MIN__
	#define WCHAR_MIN __WCHAR_MIN__
	#elif defined(__WCHAR_UNSIGNED__) || (L'\0' - 1 > 0)
	#define WCHAR_MIN (0 + L'\0')
	#else
	#define WCHAR_MIN (-0x7fffffff - 1 + L'\0')
	#endif
#endif

	/* WCHAR_MAX definition. */
#ifndef WCHAR_MAX
	#ifdef __WCHAR_MAX__
	#define WCHAR_MAX __WCHAR_MAX__
	#elif defined(__WCHAR_UNSIGNED__) || (L'\0' - 1 > 0)
	#define WCHAR_MAX (0xffffffffu + L'\0')
	#else
	#define WCHAR_MAX (0x7fffffff + L'\0')
	#endif
#endif

	/* As in stdio.h, <sys/reent.h> defines __FILE. */
#ifndef FILE_T
	#define FILE_T
	typedef __FILE FILE;
#endif

	struct tm;
#ifndef _MBSTATE_T
	#define _MBSTATE_T
	typedef _mbstate_t mbstate_t;
#endif /* _MBSTATE_T */

	/**
	 * @defgroup wcharlib Wchar library.
	 * 
	 * @brief Wide-character operations.
	 */
	/**@{*/

	/* Forward declarations. */
	extern wint_t btowc(int);
	extern wint_t fgetwc(__FILE *);
	extern wchar_t *fgetws(wchar_t *restrict, int, __FILE *restrict);
	extern wint_t fputwc(wchar_t, __FILE *);
	extern int fputws(const wchar_t *restrict, __FILE *restrict);
	extern int fwide(__FILE *, int);
	extern int fwprintf(__FILE *restrict, const wchar_t *restrict, ...);
	extern int fwscanf(__FILE *restrict, const wchar_t *restrict, ...);
	extern wint_t getwc(__FILE *);
	extern wint_t getwchar(void);
	extern size_t mbrlen(const char *restrict, size_t, mbstate_t *restrict);
	extern size_t mbrtowc(wchar_t *restrict, const char *restrict, size_t, 
		mbstate_t *restrict);
	extern int mbsinit(const mbstate_t *);
	extern size_t mbsrtowcs(wchar_t *restrict, const char **restrict, size_t, 
		mbstate_t *restrict);
	extern wint_t putwc(wchar_t, __FILE *);
	extern wint_t putwchar(wchar_t);
	extern int swprintf(wchar_t *restrict, size_t, const wchar_t *restrict, ...);
	extern int swscanf(const wchar_t *restrict, const wchar_t *restrict, ...);
	extern wint_t ungetwc(wint_t, __FILE *);
	extern int vfwprintf(__FILE *restrict, const wchar_t *restrict, va_list);
	extern int vfwscanf(__FILE *restrict, const wchar_t *restrict, va_list);
	extern int vswprintf(wchar_t *restrict, size_t, const wchar_t *restrict, 
		va_list);
	extern int vswscanf(const wchar_t *restrict, const wchar_t *restrict, 
		va_list);
	extern int vwprintf(const wchar_t *restrict, va_list);
	extern int vwscanf(const wchar_t *restrict, va_list);
	extern size_t wcrtomb(char *restrict, wchar_t, mbstate_t *restrict);
	extern wchar_t *wcscat(wchar_t *restrict, const wchar_t *restrict);
	extern wchar_t *wcschr(const wchar_t *, wchar_t);
	extern int wcscmp(const wchar_t *, const wchar_t *);
	extern int wcscoll(const wchar_t *, const wchar_t *);
	extern wchar_t *wcscpy(wchar_t *restrict, const wchar_t *restrict);
	extern size_t wcscspn(const wchar_t *, const wchar_t *);
	extern size_t wcsftime(wchar_t *restrict, size_t, const wchar_t *restrict, 
		const struct tm *restrict);
	extern size_t wcslen(const wchar_t *);
	extern wchar_t *wcsncat(wchar_t *restrict, const wchar_t *restrict, size_t);
	extern int wcsncmp(const wchar_t *, const wchar_t *, size_t);
	extern wchar_t *wcsncpy(wchar_t *restrict, const wchar_t *restrict, size_t);
	extern wchar_t *wcspbrk(const wchar_t *, const wchar_t *);
	extern wchar_t *wcsrchr(const wchar_t *, wchar_t);
	extern size_t wcsrtombs(char *restrict, const wchar_t **restrict, size_t, 
		mbstate_t *restrict);
	extern size_t wcsspn(const wchar_t *, const wchar_t *);
	extern wchar_t *wcsstr(const wchar_t *restrict, const wchar_t *restrict);
	extern double wcstod(const wchar_t *restrict, wchar_t **restrict);
	extern float wcstof(const wchar_t *restrict, wchar_t **restrict);
	extern wchar_t *wcstok(wchar_t *restrict, const wchar_t *restrict, 
		wchar_t **restrict);
	extern long wcstol(const wchar_t *restrict, wchar_t **restrict, int);
	extern long double wcstold(const wchar_t *restrict, wchar_t **restrict);
	extern long long wcstoll(const wchar_t *restrict, wchar_t **restrict, int);
	extern unsigned long wcstoul(const wchar_t *restrict, wchar_t **restrict,
		int);
	extern unsigned long long wcstoull(const wchar_t *restrict, 
		wchar_t **restrict, int);
	extern size_t wcsxfrm(wchar_t *restrict, const wchar_t *restrict, size_t);
	extern int wctob(wint_t);
	extern wchar_t *wmemchr(const wchar_t *, wchar_t, size_t);
	extern int wmemcmp(const wchar_t *, const wchar_t *, size_t);
	extern wchar_t *wmemcpy(wchar_t *restrict, const wchar_t *restrict, size_t);
	extern wchar_t *wmemmove(wchar_t *, const wchar_t *, size_t);
	extern wchar_t *wmemset(wchar_t *, wchar_t, size_t);
	extern int wprintf(const wchar_t *restrict, ...);
	extern int wscanf(const wchar_t *restrict, ...);

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)
	extern size_t mbsnrtowcs(wchar_t *restrict, const char **restrict, size_t, 
		size_t, mbstate_t *restrict);
	extern __FILE *open_wmemstream(wchar_t **, size_t *);
	extern wchar_t *wcpcpy(wchar_t *restrict, const wchar_t *restrict);
	extern wchar_t *wcpncpy(wchar_t *restrict, const wchar_t *restrict, size_t);
	extern int wcscasecmp(const wchar_t *, const wchar_t *);
	extern wchar_t *wcsdup(const wchar_t *);
	extern int wcsncasecmp(const wchar_t *, const wchar_t *, size_t);
	extern size_t wcsnlen(const wchar_t *, size_t);
	extern size_t wcsnrtombs(char *restrict, const wchar_t **restrict, size_t, 
		size_t, mbstate_t *restrict);
#endif

#ifdef _XOPEN_SOURCE
	extern int wcswidth(const wchar_t *, size_t);
	extern int wcwidth(wchar_t);
#endif

	/* Newlib needs to those non-standard functions. */
	extern size_t wcslcpy(wchar_t *, const wchar_t *, size_t);
	extern wint_t _fgetwc_r(struct _reent *, __FILE *);
	extern wint_t _fputwc_r(struct _reent *, wchar_t, __FILE *);
	extern size_t _mbrtowc_r(struct _reent *, wchar_t *, const char *, size_t, 
		mbstate_t *);
	extern size_t _mbsnrtowcs_r(struct _reent *, wchar_t *, const char **, 
		size_t, size_t, mbstate_t *);
	extern wint_t _ungetwc_r(struct _reent *, wint_t wc, __FILE *);
	extern int _vfwprintf_r(struct _reent *, __FILE *, const wchar_t *, va_list);
	extern int _vfwscanf_r(struct _reent *, __FILE *, const wchar_t *, va_list);
	extern int _vswscanf_r(struct _reent *, const wchar_t *, const wchar_t *, 
		va_list);
	extern size_t _wcrtomb_r(struct _reent *, char *, wchar_t, mbstate_t *);
	extern size_t _wcsnrtombs_r(struct _reent *, char *, const wchar_t **, 
		size_t, size_t, mbstate_t *);
	extern size_t _wcsrtombs_r(struct _reent *, char *, const wchar_t **, 
		size_t, mbstate_t *);
	extern double _wcstod_r(struct _reent *, const wchar_t *, wchar_t **);
	extern long _wcstol_r(struct _reent *, const wchar_t *, wchar_t **, int);
	extern long long _wcstoll_r(struct _reent *, const wchar_t *, wchar_t **,
		int);
	extern unsigned long _wcstoul_r(struct _reent *, const wchar_t *, wchar_t **,
		int);
	extern unsigned long long _wcstoull_r(struct _reent *, const wchar_t *, 
		wchar_t **, int);

	/**@}*/

#endif /* WCHAR_H_ */

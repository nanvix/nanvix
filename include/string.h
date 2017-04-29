/*
 * Copyright(C) 2017 Davidson Francis <davidsondfgl@gmail.com>
 8                   Pedro H. Penna   <pedrohenriquepenna@gmail.com>
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

/*
 * Copyright (c) 1994-2009  Red Hat, Inc. All rights reserved.
 *
 * This copyrighted material is made available to anyone wishing to use,
 * modify, copy, or redistribute it subject to the terms and conditions
 * of the BSD License.   This program is distributed in the hope that
 * it will be useful, but WITHOUT ANY WARRANTY expressed or implied,
 * including the implied warranties of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE.  A copy of this license is available at 
 * http://www.opensource.org/licenses. Any Red Hat trademarks that are
 * incorporated in the source code or documentation are not subject to
 * the BSD License and may only be used or replicated with the express
 * permission of Red Hat, Inc.
 */

/*
 * Copyright (c) 1981-2000 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without modification,
 * are permitted provided that the following conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright notice, 
 *       this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright notice,
 *       this list of conditions and the following disclaimer in the documentation
 *       and/or other materials provided with the distribution.
 *     * Neither the name of the University nor the names of its contributors 
 *       may be used to endorse or promote products derived from this software 
 *       without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT,
 * INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 */

/**
 * @file
 * 
 * @brief String operations library.
 */

#ifndef STRING_H_
#define	STRING_H_

	#include "_ansi.h"
	#include <sys/reent.h>
	#include <sys/cdefs.h>
	#include <sys/features.h>

	#define __need_size_t
	#define __need_NULL
	
#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)
	#define _NEED_LOCALE_T
	#include <stddef.h>
#endif

	#include <decl.h>

	/**
	 * @defgroup stringlib String Library
	 * 
	 * @brief String operations.
	 */
	/**@{*/

	extern void *memchr(const void *, int, size_t);
	extern int memcmp(const void *, const void *, size_t);
	extern void *memcpy(void *restrict, const void *restrict, size_t);
	extern void *memmove(void *, const void *, size_t);
	extern void *memset(void *, int, size_t);
	extern char *strcat(char *restrict, const char *restrict);
	extern char *strchr(const char *, int);
	extern int strcmp(const char *, const char *);
	extern int strcoll(const char *, const char *);
	extern char *strcpy(char *restrict, const char *restrict);
	extern size_t strcspn(const char *, const char *);
	extern char *strerror(int);
	extern size_t strlen(const char *);
	extern char *strncat(char *restrict, const char *restrict, size_t);
	extern int strncmp(const char *, const char *, size_t);
	extern char *strncpy(char *restrict, const char *restrict, size_t);
	extern char *strpbrk(const char *, const char *);
	extern char *strrchr(const char *, int);
	extern size_t strspn(const char *, const char *);
	extern char *strstr(const char *, const char *);
	extern char *strtok(char *restrict, const char *restrict);
	extern size_t strxfrm(char *restrict, const char *restrict, size_t);

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)
	extern char *stpcpy(char *restrict, const char *restrict);
	extern char *stpncpy(char *restrict, const char *restrict, size_t);
	extern char *strdup(const char *);
	extern char *_strdup_r(struct _reent *, const char *);
	extern int strerror_r(int, char *, size_t);
	extern char *strndup(const char *, size_t);
	extern char *_strndup_r(struct _reent *, const char *, size_t);
	extern size_t strnlen(const char *, size_t);
	extern char *strsignal(int);
	extern char *strtok_r(char *restrict, const char *restrict, char **restrict);
#endif

#ifdef _XOPEN_SOURCE
	extern void *memccpy(void *restrict, const void *restrict, int, size_t);
#endif

	/* Newlib needs to those non-standard functions. */
	extern char *_strerror_r(struct _reent *, int, int, int *);
	extern size_t strlcpy(char *, const char *, size_t);

	/**@}*/

#endif /* STRING_H_ */

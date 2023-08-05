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
 * @brief Common declarations library.
 */

#ifdef _NEED_LOCALE_T
#ifndef _LOCALE_T
#define _LOCALE_T

	/**
	 * @brief Used for locale objects.
	 */
	typedef unsigned locale_t;

#endif
#endif

#ifdef _NEED_NULL
#ifndef _NULL
#define _NULL

	/**
	 * @brief Null pointer.
	 */
	#define NULL ((void *) 0)

#endif
#endif

#ifdef _NEED_SIZE_T
#ifndef SIZE_T
#define SIZE_T

	/**
	 * @brief Used for sizes of objects.
	 */
	typedef unsigned size_t;

#endif
#endif

#ifdef _NEED_WCHAR_T
#ifndef _WCHAR_T
#define _WCHAR_T

	/**
	 * @brief Codes for all members of the largest extended character set.
	 */
	typedef unsigned wchar_t;

#endif
#endif

#ifdef _NEED_WSTATUS
#ifndef _WSTATUS
#define _WSTATUS

	#define WEXITSTATUS(status) \
		(status & 0xff)         \

    #define WIFEXITED(status) \
		((status >> 8) & 1)   \

    #define WIFSIGNALED(status) \
		((status >> 9) & 1)     \

	#define WIFSTOPPED(status) \
		(((status) >> 10) & 1) \

    #define WTERMSIG(status)    \
		((status >> 16) & 0xff) \

#endif
#endif

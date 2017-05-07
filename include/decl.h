/*
 * Copyright(C) 2011-2017 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2017 Davidson Francis <davidsondfgl@gmail.com>
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

#ifdef __need_NULL
#ifndef __null
#define __null

	/**
	 * @brief Null pointer.
	 */
	#define NULL ((void *) 0)

#endif
#endif


#ifdef __need_size_t
#ifndef __size_t
#define __size_t

	/**
	 * @brief Used for sizes of objects.
	 */
	typedef unsigned size_t;

#endif
#endif

#ifdef __need_wchar_t
#ifndef _wchar_t	
#define _wchar_t

	/**
	 * @brief Codes for all members of the largest extended character set.
	 */
	typedef signed wchar_t;

#endif
#endif


#ifdef __need_wint_t
#ifndef _wint_t	
#define _wint_t

	/**
	 * @brief An integral type capable of storing any valid value of wchar_t, or WEOF.
	 */
	typedef unsigned wint_t;

#endif
#endif


#ifdef __need_ptrdiff_t
#ifndef _ptrdiff_t
#define _ptrdiff_t

	/**
	 * @brief Signed integer type of the result of subtracting two pointers.
	 */
	typedef signed ptrdiff_t;

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

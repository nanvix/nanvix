/*
 * Copyright(C) 2017 Davidson Francis <davidsondfgl@gmail.com>
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

/* ANSI C namespace clean utility typedefs */

/* This file defines various typedefs needed by the system calls that support
   the C library.  Basically, they're just the POSIX versions with an '_'
   prepended.  This file lives in the `sys' directory so targets can provide
   their own if desired (or they can put target dependant conditionals here).
*/

#ifndef	_SYS__TYPES_H
#define _SYS__TYPES_H

#include <machine/_types.h>
#include <sys/lock.h>

#ifndef __off_t_defined
typedef signed _off_t;
#endif

#ifndef __dev_t_defined
typedef unsigned __dev_t;
#endif

#ifndef __uid_t_defined
typedef int __uid_t;
#endif

#ifndef __id_t_defined
typedef int __id_t;
#endif

#ifndef __gid_t_defined
typedef int __gid_t;
#endif

#ifndef __off64_t_defined
__extension__ typedef long long _off64_t;
#endif

/*
 * We need fpos_t for the following, but it doesn't have a leading "_",
 * so we use _fpos_t instead.
 */
#ifndef __fpos_t_defined
typedef long _fpos_t;		/* XXX must match off_t in <sys/types.h> */
				/* (and must be `long' for now) */
#endif

#ifdef __LARGE64_FILES
#ifndef __fpos64_t_defined
typedef _off64_t _fpos64_t;
#endif
#endif

#ifndef __ssize_t_defined
#ifdef __SIZE_TYPE__
/* If __SIZE_TYPE__ is defined (gcc) we define ssize_t based on size_t.
   We simply change "unsigned" to "signed" for this single definition
   to make sure ssize_t and size_t only differ by their signedness. */
#define unsigned signed
typedef __SIZE_TYPE__ _ssize_t;
#undef unsigned
#else
#if defined(__INT_MAX__) && __INT_MAX__ == 2147483647
typedef int _ssize_t;
#else
typedef long _ssize_t;
#endif
#endif
#endif

#define __need_wint_t
#include <stddef.h>

#ifndef __mbstate_t_defined
/* Conversion state information.  */
typedef struct
{
  int __count;
  union
  {
    wint_t __wch;
    unsigned char __wchb[4];
  } __value;		/* Value so far.  */
} _mbstate_t;
#endif

#ifndef __flock_t_defined
typedef _LOCK_RECURSIVE_T _flock_t;
#endif

#ifndef __iconv_t_defined
/* Iconv descriptor type */
typedef void *_iconv_t;
#endif

#endif	/* _SYS__TYPES_H */

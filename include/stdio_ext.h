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

/*
 * stdio_ext.h
 *
 * Definitions for I/O internal operations, originally from Solaris.
 */

#ifndef _STDIO_EXT_H_
#define _STDIO_EXT_H_

#ifdef __rtems__
#error "<stdio_ext.h> not supported"
#endif

#include <stdio.h>

#define	FSETLOCKING_QUERY	0
#define	FSETLOCKING_INTERNAL	1
#define	FSETLOCKING_BYCALLER	2

_BEGIN_STD_C

void	 _EXFUN(__fpurge,(FILE *));
int	 _EXFUN(__fsetlocking,(FILE *, int));

/* TODO:

   void _flushlbf (void);
*/

#ifdef  __GNUC__

_ELIDABLE_INLINE size_t
__fbufsize (FILE *__fp) { return (size_t) __fp->_bf._size; }

_ELIDABLE_INLINE int
__freading (FILE *__fp) { return (__fp->_flags & __SRD) != 0; }

_ELIDABLE_INLINE int
__fwriting (FILE *__fp) { return (__fp->_flags & __SWR) != 0; }

_ELIDABLE_INLINE int
__freadable (FILE *__fp) { return (__fp->_flags & (__SRD | __SRW)) != 0; }

_ELIDABLE_INLINE int
__fwritable (FILE *__fp) { return (__fp->_flags & (__SWR | __SRW)) != 0; }

_ELIDABLE_INLINE int
__flbf (FILE *__fp) { return (__fp->_flags & __SLBF) != 0; }

_ELIDABLE_INLINE size_t
__fpending (FILE *__fp) { return __fp->_p - __fp->_bf._base; }

#else

size_t	 _EXFUN(__fbufsize,(FILE *));
int	 _EXFUN(__freading,(FILE *));
int	 _EXFUN(__fwriting,(FILE *));
int	 _EXFUN(__freadable,(FILE *));
int	 _EXFUN(__fwritable,(FILE *));
int	 _EXFUN(__flbf,(FILE *));
size_t	 _EXFUN(__fpending,(FILE *));

#ifndef __cplusplus

#define __fbufsize(__fp) ((size_t) (__fp)->_bf._size)
#define __freading(__fp) (((__fp)->_flags & __SRD) != 0)
#define __fwriting(__fp) (((__fp)->_flags & __SWR) != 0)
#define __freadable(__fp) (((__fp)->_flags & (__SRD | __SRW)) != 0)
#define __fwritable(__fp) (((__fp)->_flags & (__SWR | __SRW)) != 0)
#define __flbf(__fp) (((__fp)->_flags & __SLBF) != 0)
#define __fpending(__fp) ((size_t) ((__fp)->_p - (__fp)->_bf._base))

#endif /* __cplusplus */

#endif /* __GNUC__ */

_END_STD_C

#endif /* _STDIO_EXT_H_ */

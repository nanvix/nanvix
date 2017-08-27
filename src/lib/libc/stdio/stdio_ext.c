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
FUNCTION
<<stdio_ext>>,<<__fbufsize>>,<<__fpending>>,<<__flbf>>,<<__freadable>>,<<__fwritable>>,<<__freading>>,<<__fwriting>>---access internals of FILE structure

INDEX
	__fbufsize
INDEX
	__fpending
INDEX
	__flbf
INDEX
	__freadable
INDEX
	__fwritable
INDEX
	__freading
INDEX
	__fwriting

ANSI_SYNOPSIS
	#include <stdio.h>
	#include <stdio_ext.h>
	size_t __fbufsize(FILE *<[fp]>);
	size_t __fpending(FILE *<[fp]>);
	int __flbf(FILE *<[fp]>);
	int __freadable(FILE *<[fp]>);
	int __fwritable(FILE *<[fp]>);
	int __freading(FILE *<[fp]>);
	int __fwriting(FILE *<[fp]>);

DESCRIPTION
These functions provides access to the internals of the FILE structure <[fp]>.

RETURNS
<<__fbufsize>> returns the number of bytes in the buffer of stream <[fp]>.

<<__fpending>> returns the number of bytes in the output buffer of stream <[fp]>.

<<__flbf>> returns nonzero if stream <[fp]> is line-buffered, and <<0>> if not.

<<__freadable>> returns nonzero if stream <[fp]> may be read, and <<0>> if not.

<<__fwritable>> returns nonzero if stream <[fp]> may be written, and <<0>> if not.

<<__freading>> returns nonzero if stream <[fp]> if the last operation on
it was a read, or if it read-only, and <<0>> if not.

<<__fwriting>> returns nonzero if stream <[fp]> if the last operation on
it was a write, or if it write-only, and <<0>> if not.

PORTABILITY
These functions originate from Solaris and are also provided by GNU libc.

No supporting OS subroutines are required.
*/

#ifndef __rtems__

#include <_ansi.h>
#include <stdio.h>

/* Subroutine versions of the inline or macro functions. */

size_t
_DEFUN(__fbufsize, (fp),
       FILE * fp)
{
  return (size_t) fp->_bf._size;
}

size_t
_DEFUN(__fpending, (fp),
       FILE * fp)
{
  return fp->_p - fp->_bf._base;
}

int
_DEFUN(__flbf, (fp),
       FILE * fp)
{
  return (fp->_flags & __SLBF) != 0;
}

int
_DEFUN(__freadable, (fp),
       FILE * fp)
{
  return (fp->_flags & (__SRD | __SRW)) != 0;
}

int
_DEFUN(__fwritable, (fp),
       FILE * fp)
{
  return (fp->_flags & (__SWR | __SRW)) != 0;
}

int
_DEFUN(__freading, (fp),
       FILE * fp)
{
  return (fp->_flags & __SRD) != 0;
}

int
_DEFUN(__fwriting, (fp),
       FILE * fp)
{
  return (fp->_flags & __SWR) != 0;
}

#endif /* __rtems__ */

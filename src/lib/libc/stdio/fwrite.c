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

/*
 * Copyright (c) 1990 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms are permitted
 * provided that the above copyright notice and this paragraph are
 * duplicated in all such forms and that any documentation,
 * advertising materials, and other materials related to such
 * distribution and use acknowledge that the software was developed
 * by the University of California, Berkeley.  The name of the
 * University may not be used to endorse or promote products derived
 * from this software without specific prior written permission.
 * THIS SOFTWARE IS PROVIDED ``AS IS'' AND WITHOUT ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, WITHOUT LIMITATION, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE.
 */

#if defined(LIBC_SCCS) && !defined(lint)
static char sccsid[] = "%W% (Berkeley) %G%";
#endif /* LIBC_SCCS and not lint */

#include <_ansi.h>
#include <stdio.h>
#include <string.h>
#if 0
#include <sys/stdc.h>
#endif
#include "local.h"
#if 1
#include "fvwrite.h"
#endif

#ifdef __IMPL_UNLOCKED__
#define _fwrite_r _fwrite_unlocked_r
#define fwrite fwrite_unlocked
#endif

/*
 * Write `count' objects (each size `size') from memory to the given file.
 * Return the number of whole objects written.
 */

size_t _fwrite_r(struct _reent *ptr, const void *restrict buf, size_t size,
  size_t count, FILE *restrict fp)
{
  size_t n;
#ifdef _FVWRITE_IN_STREAMIO
  struct __suio uio;
  struct __siov iov;

  iov.iov_base = buf;
  uio.uio_resid = iov.iov_len = n = count * size;
  uio.uio_iov = &iov;
  uio.uio_iovcnt = 1;

  /*
   * The usual case is success (__sfvwrite_r returns 0);
   * skip the divide if this happens, since divides are
   * generally slow and since this occurs whenever size==0.
   */

  CHECK_INIT(ptr, fp);

  _newlib_flockfile_start (fp);
  ORIENT (fp, -1);
  if (__sfvwrite_r (ptr, fp, &uio) == 0)
    {
      _newlib_flockfile_exit (fp);
      return count;
    }
  _newlib_flockfile_end (fp);
  return (n - uio.uio_resid) / size;
#else
  size_t i = 0;
  _CONST char *p = buf;
  n = count * size;
  CHECK_INIT (ptr, fp);

  _newlib_flockfile_start (fp);
  ORIENT (fp, -1);
  /* Make sure we can write.  */
  if (cantwrite (ptr, fp))
    goto ret;

  while (i < n)
    {
      if (__sputc_r (ptr, p[i], fp) == EOF)
	break;

      i++;
    }

ret:
  _newlib_flockfile_end (fp);
  return i / size;
#endif
}

#ifndef _REENT_ONLY

/**
 * @brief Binary output.
 *
 * @details Writes, from the array pointed to by @p buf,
 * up to @p count elements whose size is specified by 
 * @p size, to the stream pointed to by @p fp.
 *
 * @return Returns the number of elements successfully written
 * which may be less than @p count if a write error is encountered.
 * If @p size or @p count is 0, fwrite() returns 0 and the state of
 * the stream remains unchanged. Otherwise, if a write error occurs,
 * the error indicator for the stream is set, and errno is set to
 * indicate the error.
 */
size_t fwrite(const void *restrict buf, size_t size, size_t count, FILE *fp)
{
  return _fwrite_r (_REENT, buf, size, count, fp);
}

#endif

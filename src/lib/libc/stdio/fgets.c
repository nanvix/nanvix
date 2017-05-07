/*
 * Copyright(C) 2016-2017 Davidson Francis <davidsondfgl@gmail.com>
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

#include <_ansi.h>
#include <stdio.h>
#include <string.h>
#include "local.h"

#ifdef __IMPL_UNLOCKED__
#define _fgets_r _fgets_unlocked_r
#define fgets fgets_unlocked
#endif

/*
 * Read at most n-1 characters from the given file.
 * Stop when a newline has been read, or the count runs out.
 * Return first argument, or NULL if no characters were read.
 */

char *_fgets_r(struct _reent * ptr, char *restrict buf, int n,
	FILE *restrict fp)
{
  size_t len;
  char *s;
  unsigned char *p, *t;

  if (n < 2)			/* sanity check */
    return 0;

  s = buf;

  CHECK_INIT(ptr, fp);

  _newlib_flockfile_start (fp);
#ifdef __SCLE
  if (fp->_flags & __SCLE)
    {
      int c = 0;
      /* Sorry, have to do it the slow way */
      while (--n > 0 && (c = __sgetc_r (ptr, fp)) != EOF)
	{
	  *s++ = c;
	  if (c == '\n')
	    break;
	}
      if (c == EOF && s == buf)
        {
          _newlib_flockfile_exit (fp);
          return NULL;
        }
      *s = 0;
      _newlib_flockfile_exit (fp);
      return buf;
    }
#endif

  n--;				/* leave space for NUL */
  do
    {
      /*
       * If the buffer is empty, refill it.
       */
      if ((len = fp->_r) <= 0)
	{
	  if (__srefill_r (ptr, fp))
	    {
	      /* EOF: stop with partial or no line */
	      if (s == buf)
                {
                  _newlib_flockfile_exit (fp);
                  return 0;
                }
	      break;
	    }
	  len = fp->_r;
	}
      p = fp->_p;

      /*
       * Scan through at most n bytes of the current buffer,
       * looking for '\n'.  If found, copy up to and including
       * newline, and stop.  Otherwise, copy entire chunk
       * and loop.
       */
      if (len > (size_t)n)
	len = n;
      t = (unsigned char *) memchr ((_PTR) p, '\n', len);
      if (t != 0)
	{
	  len = ++t - p;
	  fp->_r -= len;
	  fp->_p = t;
	  _CAST_VOID memcpy ((_PTR) s, (_PTR) p, len);
	  s[len] = 0;
          _newlib_flockfile_exit (fp);
	  return (buf);
	}
      fp->_r -= len;
      fp->_p += len;
      _CAST_VOID memcpy ((_PTR) s, (_PTR) p, len);
      s += len;
    }
  while ((n -= len) != 0);
  *s = 0;
  _newlib_flockfile_end (fp);
  return buf;
}

#ifndef _REENT_ONLY

/**
 * @brief Gets a string from a stream.
 *
 * @details Reads bytes from stream into the array
 * pointed to by @p buf until @p n -1 bytes are read,
 * or a <newline> is read and transferred to @p buf,
 * or an end-of-file condition is encountered. A null
 * byte is written immediately after the last byte read
 * into the array. If the end-of-file condition is 
 * encountered before any bytes are read, the contents of
 * the array pointed to by @p buf is not be changed.
 *
 * @return Returns @p buf. If the stream is at end-of-file,
 * the end-of-file indicator for the stream is set and fgets()
 * returns a null pointer. If a read error occurs, the error
 * indicator for the stream is set, fgets() returns a null pointer,
 * and sets errno to indicate the error.
 */
char *fgets(char *restrict buf, int n, FILE *restrict fp)
{
  return _fgets_r (_REENT, buf, n, fp);
}

#endif /* !_REENT_ONLY */

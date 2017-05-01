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
 * Copyright (c) 1990, 2007 The Regents of the University of California.
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
#include <malloc.h>
#include "local.h"

#ifdef __IMPL_UNLOCKED__
#define _fread_r _fread_unlocked_r
#define fread fread_unlocked
#endif

#ifdef __SCLE
static size_t crlf_r(struct _reent *ptr, FILE *fp, char *buf, size_t count,
	int eof)
{
  int r;
  char *s, *d, *e;

  if (count == 0)
    return 0;

  e = buf + count;
  for (s=d=buf; s<e-1; s++)
    {
      if (*s == '\r' && s[1] == '\n')
        s++;
      *d++ = *s;
    }
  if (s < e)
    {
      if (*s == '\r')
        {
          int c = __sgetc_raw_r (ptr, fp);
          if (c == '\n')
            *s = '\n';
          else
            ungetc (c, fp);
        }
      *d++ = *s++;
    }


  while (d < e)
    {
      r = _getc_r (ptr, fp);
      if (r == EOF)
        return count - (e-d);
      *d++ = r;
    }

  return count;
  
}

#endif

size_t _fread_r(struct _reent *ptr, void *restrict buf, size_t size,
	size_t count, FILE *restrict fp)
{
  register size_t resid;
  register char *p;
  register int r;
  size_t total;

  if ((resid = count * size) == 0)
    return 0;

  CHECK_INIT(ptr, fp);

  _newlib_flockfile_start (fp);
  ORIENT (fp, -1);
  if (fp->_r < 0)
    fp->_r = 0;
  total = resid;
  p = buf;

#if !defined(PREFER_SIZE_OVER_SPEED) && !defined(__OPTIMIZE_SIZE__)

  /* Optimize unbuffered I/O.  */
  if (fp->_flags & __SNBF)
    {
      /* First copy any available characters from ungetc buffer.  */
      int copy_size = resid > (size_t)fp->_r ? fp->_r : (int)resid;
      _CAST_VOID memcpy ((_PTR) p, (_PTR) fp->_p, (size_t) copy_size);
      fp->_p += copy_size;
      fp->_r -= copy_size;
      p += copy_size;
      resid -= copy_size;

      /* If still more data needed, free any allocated ungetc buffer.  */
      if (HASUB (fp) && resid > 0)
	FREEUB (ptr, fp);

      /* Finally read directly into user's buffer if needed.  */
      while (resid > 0)
	{
	  int rc = 0;
	  /* save fp buffering state */
	  void *old_base = fp->_bf._base;
	  void * old_p = fp->_p;
	  int old_size = fp->_bf._size;
	  /* allow __refill to use user's buffer */
	  fp->_bf._base = (unsigned char *) p;
	  fp->_bf._size = resid;
	  fp->_p = (unsigned char *) p;
	  rc = __srefill_r (ptr, fp);
	  /* restore fp buffering back to original state */
	  fp->_bf._base = old_base;
	  fp->_bf._size = old_size;
	  fp->_p = old_p;
	  resid -= fp->_r;
	  p += fp->_r;
	  fp->_r = 0;
	  if (rc)
	    {
#ifdef __SCLE
              if (fp->_flags & __SCLE)
	        {
	          _newlib_flockfile_exit (fp);
	          return crlf_r (ptr, fp, buf, total-resid, 1) / size;
	        }
#endif
	      _newlib_flockfile_exit (fp);
	      return (total - resid) / size;
	    }
	}
    }
  else
#endif /* !PREFER_SIZE_OVER_SPEED && !__OPTIMIZE_SIZE__ */
    {
      while (resid > (size_t)(r = fp->_r))
	{
	  _CAST_VOID memcpy ((_PTR) p, (_PTR) fp->_p, (size_t) r);
	  fp->_p += r;
	  /* fp->_r = 0 ... done in __srefill */
	  p += r;
	  resid -= r;
	  if (__srefill_r (ptr, fp))
	    {
	      /* no more input: return partial result */
#ifdef __SCLE
	      if (fp->_flags & __SCLE)
		{
		  _newlib_flockfile_exit (fp);
		  return crlf_r (ptr, fp, buf, total-resid, 1) / size;
		}
#endif
	      _newlib_flockfile_exit (fp);
	      return (total - resid) / size;
	    }
	}
      _CAST_VOID memcpy ((_PTR) p, (_PTR) fp->_p, resid);
      fp->_r -= resid;
      fp->_p += resid;
    }

  /* Perform any CR/LF clean-up if necessary.  */
#ifdef __SCLE
  if (fp->_flags & __SCLE)
    {
      _newlib_flockfile_exit (fp);
      return crlf_r(ptr, fp, buf, total, 0) / size;
    }
#endif
  _newlib_flockfile_end (fp);
  return count;
}

#ifndef _REENT_ONLY

/**
 * @brief Binary input.
 *
 * @details Reads into the array pointed to by @p buf
 * up to @p count elements whose size is specified by
 * @p size in bytes, from the stream pointed to by @p fp.
 *
 * @return Returns the number of elements successfully read
 * which is less than @p count only if a read error or end-of-file
 * is encountered. If @p size or @p count is 0, fread() returns 0
 * and the contents of the array and the state of the stream remain
 * unchanged. Otherwise, if a read error occurs, the error indicator
 * for the stream is set, and errno is set to indicate the error.
 */
size_t fread(void *restrict buf, size_t size, size_t count, FILE *restrict fp)
{
   return _fread_r (_REENT, buf, size, count, fp);
}

#endif

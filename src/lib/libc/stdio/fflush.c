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
#include <errno.h>
#include "local.h"

#ifdef __IMPL_UNLOCKED__
#define _fflush_r _fflush_unlocked_r
#define fflush fflush_unlocked
#endif

#ifndef __IMPL_UNLOCKED__
/* Flush a single file, or (if fp is NULL) all files.  */

/* Core function which does not lock file pointer.  This gets called
   directly from __srefill. */
int __sflush_r(struct _reent *ptr, register FILE * fp)
{
  register unsigned char *p;
  register _READ_WRITE_BUFSIZE_TYPE n;
  register _READ_WRITE_RETURN_TYPE t;
  short flags;

  flags = fp->_flags;
  if ((flags & __SWR) == 0)
    {
#ifdef _FSEEK_OPTIMIZATION
      /* For a read stream, an fflush causes the next seek to be
         unoptimized (i.e. forces a system-level seek).  This conforms
         to the POSIX and SUSv3 standards.  */
      fp->_flags |= __SNPT;
#endif

      /* For a seekable stream with buffered read characters, we will attempt
         a seek to the current position now.  A subsequent read will then get
         the next byte from the file rather than the buffer.  This conforms
         to the POSIX and SUSv3 standards.  Note that the standards allow
         this seek to be deferred until necessary, but we choose to do it here
         to make the change simpler, more contained, and less likely
         to miss a code scenario.  */
      if ((fp->_r > 0 || fp->_ur > 0) && fp->_seek != NULL)
	{
	  int tmp_errno;
#ifdef __LARGE64_FILES
	  _fpos64_t curoff;
#else
	  _fpos_t curoff;
#endif

	  /* Save last errno and set errno to 0, so we can check if a device
	     returns with a valid position -1.  We restore the last errno if
	     no other error condition has been encountered. */
	  tmp_errno = ptr->_errno;
	  ptr->_errno = 0;
	  /* Get the physical position we are at in the file.  */
	  if (fp->_flags & __SOFF)
	    curoff = fp->_offset;
	  else
	    {
	      /* We don't know current physical offset, so ask for it.
		 Only ESPIPE and EINVAL are ignorable.  */
#ifdef __LARGE64_FILES
	      if (fp->_flags & __SL64)
		curoff = fp->_seek64 (ptr, fp->_cookie, 0, SEEK_CUR);
	      else
#endif
		curoff = fp->_seek (ptr, fp->_cookie, 0, SEEK_CUR);
	      if (curoff == -1L && ptr->_errno != 0)
		{
		  int result = EOF;
		  if (ptr->_errno == ESPIPE || ptr->_errno == EINVAL)
		    {
		      result = 0;
		      ptr->_errno = tmp_errno;
		    }
		  else
		    fp->_flags |= __SERR;
		  return result;
		}
            }
          if (fp->_flags & __SRD)
            {
              /* Current offset is at end of buffer.  Compensate for
                 characters not yet read.  */
              curoff -= fp->_r;
              if (HASUB (fp))
                curoff -= fp->_ur;
            }
	  /* Now physically seek to after byte last read.  */
#ifdef __LARGE64_FILES
	  if (fp->_flags & __SL64)
	    curoff = fp->_seek64 (ptr, fp->_cookie, curoff, SEEK_SET);
	  else
#endif
	    curoff = fp->_seek (ptr, fp->_cookie, curoff, SEEK_SET);
	  if (curoff != -1 || ptr->_errno == 0
	      || ptr->_errno == ESPIPE || ptr->_errno == EINVAL)
	    {
	      /* Seek successful or ignorable error condition.
		 We can clear read buffer now.  */
#ifdef _FSEEK_OPTIMIZATION
	      fp->_flags &= ~__SNPT;
#endif
	      fp->_r = 0;
	      fp->_p = fp->_bf._base;
	      if ((fp->_flags & __SOFF) && (curoff != -1 || ptr->_errno == 0))
		fp->_offset = curoff;
	      ptr->_errno = tmp_errno;
	      if (HASUB (fp))
		FREEUB (ptr, fp);
	    }
	  else
	    {
	      fp->_flags |= __SERR;
	      return EOF;
	    }
	}
      return 0;
    }
  if ((p = fp->_bf._base) == NULL)
    {
      /* Nothing to flush.  */
      return 0;
    }
  n = fp->_p - p;		/* write this much */

  /*
   * Set these immediately to avoid problems with longjmp
   * and to allow exchange buffering (via setvbuf) in user
   * write function.
   */
  fp->_p = p;
  fp->_w = flags & (__SLBF | __SNBF) ? 0 : fp->_bf._size;

  while (n > 0)
    {
      t = fp->_write (ptr, fp->_cookie, (char *) p, n);
      if (t <= 0)
	{
          fp->_flags |= __SERR;
          return EOF;
	}
      p += t;
      n -= t;
    }
  return 0;
}

#ifdef _STDIO_BSD_SEMANTICS
/* Called from _cleanup_r.  At exit time, we don't need file locking,
   and we don't want to move the underlying file pointer unless we're
   writing. */
int __sflushw_r(struct _reent *ptr, register FILE *fp)
{
  return (fp->_flags & __SWR) ?  __sflush_r (ptr, fp) : 0;
}
#endif

#endif /* __IMPL_UNLOCKED__ */

int _fflush_r(struct _reent *ptr, register FILE *fp)
{
  int ret;

#ifdef _REENT_SMALL
  /* For REENT_SMALL platforms, it is possible we are being
     called for the first time on a std stream.  This std
     stream can belong to a reentrant struct that is not
     _REENT.  If CHECK_INIT gets called below based on _REENT,
     we will end up changing said file pointers to the equivalent
     std stream off of _REENT.  This causes unexpected behavior if
     there is any data to flush on the _REENT std stream.  There
     are two alternatives to fix this:  1) make a reentrant fflush
     or 2) simply recognize that this file has nothing to flush
     and return immediately before performing a CHECK_INIT.  Choice
     2 is implemented here due to its simplicity.  */
  if (fp->_bf._base == NULL)
    return 0;
#endif /* _REENT_SMALL  */

  CHECK_INIT (ptr, fp);

  if (!fp->_flags)
    return 0;

  _newlib_flockfile_start (fp);
  ret = __sflush_r (ptr, fp);
  _newlib_flockfile_end (fp);
  return ret;
}

#ifndef _REENT_ONLY

/**
 * @brief Flushs a stream.
 *
 * @details If @p fp points to an output stream or
 * an update stream in which the most recent operation
 * was not input, fflush() causes any unwritten data for
 * that stream to be written to the file.
 *
 * @return Returns 0; otherwise, sets the error indicator
 * for the stream, return EOF.
 */
int fflush(register FILE *fp)
{
  if (fp == NULL)
    return _fwalk_reent (_GLOBAL_REENT, _fflush_r);

  return _fflush_r (_REENT, fp);
}

#endif /* _REENT_ONLY */

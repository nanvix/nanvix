/*
 * Copyright(C) 2017-2016 Davidson Francis <davidsondfgl@gmail.com>
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
#include <reent.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/lock.h>
#include "local.h"

int _fclose_r(struct _reent *rptr, register FILE * fp)
{
  int r;

  if (fp == NULL)
    return (0);			/* on NULL */

  CHECK_INIT (rptr, fp);

  /* We can't use the _newlib_flockfile_XXX macros here due to the
     interlocked locking with the sfp_lock. */
#ifdef _STDIO_WITH_THREAD_CANCELLATION_SUPPORT
  int __oldcancel;
  pthread_setcancelstate (PTHREAD_CANCEL_DISABLE, &__oldcancel);
#endif
  _flockfile (fp);

  if (fp->_flags == 0)		/* not open! */
    {
      _funlockfile (fp);
#ifdef _STDIO_WITH_THREAD_CANCELLATION_SUPPORT
      pthread_setcancelstate (__oldcancel, &__oldcancel);
#endif
      return (0);
    }
#ifdef _STDIO_BSD_SEMANTICS
  /* BSD and Glibc systems only flush streams which have been written to. */
  r = (fp->_flags & __SWR) ? __sflush_r (rptr, fp) : 0;
#else
  /* Follow POSIX semantics exactly.  Unconditionally flush to allow
     special handling for seekable read files to reposition file to last
     byte processed as opposed to last byte read ahead into the buffer. */
  r = __sflush_r (rptr, fp);
#endif
  if (fp->_close != NULL && fp->_close (rptr, fp->_cookie) < 0)
    r = EOF;
  if (fp->_flags & __SMBF)
    _free_r (rptr, (char *) fp->_bf._base);
  if (HASUB (fp))
    FREEUB (rptr, fp);
  if (HASLB (fp))
    FREELB (rptr, fp);
  __sfp_lock_acquire ();
  fp->_flags = 0;		/* release this FILE for reuse */
  _funlockfile (fp);
#ifndef __SINGLE_THREAD__
  __lock_close_recursive (fp->_lock);
#endif

  __sfp_lock_release ();
#ifdef _STDIO_WITH_THREAD_CANCELLATION_SUPPORT
  pthread_setcancelstate (__oldcancel, &__oldcancel);
#endif

  return (r);
}

#ifndef _REENT_ONLY

/**
 * @brief Closes a stream.
 *
 * @details Causes the stream pointed to by
 * @p fp to be flushed and the associated file
 * to be closed. Any unwritten buffered data for
 * the stream is written to the file; any unread
 * buffered data is discarded.
 *
 * @return Returns 0; otherwise, it returns EOF and
 * sets errno to indicate the error.
 */
int fclose(register FILE * fp)
{
  return _fclose_r(_REENT, fp);
}

#endif

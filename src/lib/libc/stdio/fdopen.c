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
#include <reent.h>
#include <sys/types.h>
#include <sys/fcntl.h>
#include <stdio.h>
#include <errno.h>
#include "local.h"
#include <_syslist.h>

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

FILE * _fdopen_r(struct _reent *ptr, int fd, const char *mode)
{
  register FILE *fp;
  int flags, oflags;
#ifdef HAVE_FCNTL
  int fdflags, fdmode;
#endif

  if ((flags = __sflags (ptr, mode, &oflags)) == 0)
    return 0;

  /* make sure the mode the user wants is a subset of the actual mode */
#ifdef HAVE_FCNTL
  if ((fdflags = _fcntl_r (ptr, fd, F_GETFL, 0)) < 0)
    return 0;
  fdmode = fdflags & O_ACCMODE;
  if (fdmode != O_RDWR && (fdmode != (oflags & O_ACCMODE)))
    {
      ptr->_errno = EBADF;
      return 0;
    }
#endif

  if ((fp = __sfp (ptr)) == 0)
    return 0;

  _newlib_flockfile_start (fp);

  fp->_flags = flags;
  /* POSIX recommends setting the O_APPEND bit on fd to match append
     streams.  Someone may later clear O_APPEND on fileno(fp), but the
     stream must still remain in append mode.  Rely on __sflags
     setting __SAPP properly.  */
#ifdef HAVE_FCNTL
  if ((oflags & O_APPEND) && !(fdflags & O_APPEND))
    _fcntl_r (ptr, fd, F_SETFL, fdflags | O_APPEND);
#endif
  fp->_file = fd;
  fp->_cookie = (_PTR) fp;

#undef _read
#undef _write
#undef _seek
#undef _close

  fp->_read = __sread;
  fp->_write = __swrite;
  fp->_seek = __sseek;
  fp->_close = __sclose;

#ifdef __SCLE
  /* Explicit given mode results in explicit setting mode on fd */
  if (oflags & O_BINARY)
    setmode (fp->_file, O_BINARY);
  else if (oflags & O_TEXT)
    setmode (fp->_file, O_TEXT);
  if (__stextmode (fp->_file))
    fp->_flags |= __SCLE;
#endif

  _newlib_flockfile_end (fp);
  return fp;
}

#ifndef _REENT_ONLY

/**
 * @brief Associates a stream with a file descriptor.
 *
 * @details Associates a stream with the existing file
 * descriptor, @p fd. The @p mode of the stream must be
 * compatible with the mode of the file descriptor.
 *
 * @return Returns a pointer to a stream; otherwise, a
 * null pointer is returned and errno set to indicate the
 * error.
 */
FILE * fdopen(int fd, const char *mode)
{
  return _fdopen_r (_REENT, fd, mode);
}

#endif /* ! _REENT_ONLY */
#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */

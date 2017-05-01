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

#if defined(LIBC_SCCS) && !defined(lint)
static char sccsid[] = "%W% (Berkeley) %G%";
#endif /* LIBC_SCCS and not lint */

#include <_ansi.h>
#include <reent.h>
#include <stdio.h>
#include <errno.h>
#include <sys/lock.h>
#include "local.h"

FILE *_fopen_r(struct _reent *ptr, const char *restrict file,
  const char *restrict mode)
{
  register FILE *fp;
  register int f;
  int flags, oflags;

  if ((flags = __sflags (ptr, mode, &oflags)) == 0)
    return NULL;
  if ((fp = __sfp (ptr)) == NULL)
    return NULL;

  if ((f = _open_r (ptr, file, oflags, 0666)) < 0)
    {
      _newlib_sfp_lock_start (); 
      fp->_flags = 0;		/* release */
#ifndef __SINGLE_THREAD__
      __lock_close_recursive (fp->_lock);
#endif
      _newlib_sfp_lock_end (); 
      return NULL;
    }

  _newlib_flockfile_start (fp);

  fp->_file = f;
  fp->_flags = flags;
  fp->_cookie = (_PTR) fp;
  fp->_read = __sread;
  fp->_write = __swrite;
  fp->_seek = __sseek;
  fp->_close = __sclose;

  if (fp->_flags & __SAPP)
    _fseek_r (ptr, fp, 0, SEEK_END);

#ifdef __SCLE
  if (__stextmode (fp->_file))
    fp->_flags |= __SCLE;
#endif

  _newlib_flockfile_end (fp);
  return fp;
}

#ifndef _REENT_ONLY

/**
 * @brief Opens a stream.
 *
 * @details Opens the file whose pathname is the string
 * pointed to by @p file, and associates a stream with it.
 *
 * @return Returns a pointer to the object controlling the
 * stream. Otherwise, a null pointer is returned.
 */
FILE *fopen(const char *file, const char *mode)
{
  return _fopen_r (_REENT, file, mode);
}

#endif

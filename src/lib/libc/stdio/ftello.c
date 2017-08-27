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

/*
 * ftello: return current offset.
 */

#include <_ansi.h>
#include <reent.h>
#include <stdio.h>
#include <errno.h>
#include "local.h"

_off_t _ftello_r(struct _reent * ptr, register FILE * fp)
{
  _fpos_t pos;

  /* Ensure stdio is set up.  */

  CHECK_INIT (ptr, fp);

  _newlib_flockfile_start (fp);

  if (fp->_seek == NULL)
    {
      ptr->_errno = ESPIPE;
      _newlib_flockfile_exit (fp);
      return -1L;
    }

  /* Find offset of underlying I/O object, then adjust for buffered
     bytes.  Flush a write stream, since the offset may be altered if
     the stream is appending.  Do not flush a read stream, since we
     must not lose the ungetc buffer.  */
  if (fp->_flags & __SWR)
    _fflush_r (ptr, fp);
  if (fp->_flags & __SOFF)
    pos = fp->_offset;
  else
    {
      pos = fp->_seek (ptr, fp->_cookie, (_fpos_t) 0, SEEK_CUR);
      if (pos == -1L)
        {
          _newlib_flockfile_exit (fp);
          return pos;
        }
    }
  if (fp->_flags & __SRD)
    {
      /*
       * Reading.  Any unread characters (including
       * those from ungetc) cause the position to be
       * smaller than that in the underlying object.
       */
      pos -= fp->_r;
      if (HASUB (fp))
	pos -= fp->_ur;
    }
  else if ((fp->_flags & __SWR) && fp->_p != NULL)
    {
      /*
       * Writing.  Any buffered characters cause the
       * position to be greater than that in the
       * underlying object.
       */
      pos += fp->_p - fp->_bf._base;
    }

  _newlib_flockfile_end (fp);
  return pos;
}

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

#ifndef _REENT_ONLY

/**
 * @brief Returns a file offset in a stream.
 *
 * @details Same ftell behaviour except that the return
 * value is of type off_t.
 *
 * @return Returns the current value of the file-position
 * indicator for the stream measured in bytes from the 
 * beginning of the file. Otherwise, returns -1 and sets
 * errno to indicate the error.
 */
_off_t ftello(register FILE * fp)
{
  return _ftello_r (_REENT, fp);
}

#endif /* !_REENT_ONLY */
#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */

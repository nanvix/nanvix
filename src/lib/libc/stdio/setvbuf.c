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

#include <_ansi.h>
#include <stdio.h>
#include <stdlib.h>
#include "local.h"

/**
 * @brief Assigns buffering to a stream.
 *
 * @details The setvbuf() function may be used after the stream
 * pointed to by @p fp is associated with an open file but before
 * any other operation (other than an unsuccessful call to setvbuf())
 * is performed on the stream. The argument @p mode determines how
 * stream shall be buffered, as follows:
 * {_IOFBF} causes input/output to be fully buffered.
 * {_IOLBF} causes input/output to be line buffered.
 * {_IONBF} causes input/output to be unbuffered.
 * If @p buf is not a null pointer, the array it points to may be used
 * instead of a buffer allocated by setvbuf() and the argument @p size
 * specifies the size of the array; otherwise, @p size may determine the
 * size of a buffer allocated by the setvbuf() function. The contents of
 * the array at any time are unspecified.
 *
 * @return Returns 0 on success; otherwise, returns a non-zero value if an
 * invalid value is given for @p mode or if the request cannot be honored.
 */
int setvbuf(register FILE *fp, char *buf, register int mode,
  register size_t size)
{
  int ret = 0;
  struct _reent *reent = _REENT;

  CHECK_INIT (reent, fp);

  _newlib_flockfile_start (fp);

  /*
   * Verify arguments.  The `int' limit on `size' is due to this
   * particular implementation.
   */

  if ((mode != _IOFBF && mode != _IOLBF && mode != _IONBF) || (int)(_POINTER_INT) size < 0)
    {
      _newlib_flockfile_exit (fp);
      return (EOF);
    }

  /*
   * Write current buffer, if any; drop read count, if any.
   * Make sure putc() will not think fp is line buffered.
   * Free old buffer if it was from malloc().  Clear line and
   * non buffer flags, and clear malloc flag.
   */

  _fflush_r (reent, fp);
  fp->_r = 0;
  fp->_lbfsize = 0;
  if (fp->_flags & __SMBF)
    _free_r (reent, (_PTR) fp->_bf._base);
  fp->_flags &= ~(__SLBF | __SNBF | __SMBF);

  if (mode == _IONBF)
    goto nbf;

  /*
   * Allocate buffer if needed. */
  if (buf == NULL)
    {
      /* we need this here because malloc() may return a pointer
	 even if size == 0 */
      if (!size) size = BUFSIZ;
      if ((buf = malloc (size)) == NULL)
	{
	  ret = EOF;
	  /* Try another size... */
	  buf = malloc (BUFSIZ);
	  size = BUFSIZ;
	}
      if (buf == NULL)
        {
          /* Can't allocate it, let's try another approach */
nbf:
          fp->_flags |= __SNBF;
          fp->_w = 0;
          fp->_bf._base = fp->_p = fp->_nbuf;
          fp->_bf._size = 1;
          _newlib_flockfile_exit (fp);
          return (ret);
        }
      fp->_flags |= __SMBF;
    }
  /*
   * Now put back whichever flag is needed, and fix _lbfsize
   * if line buffered.  Ensure output flush on exit if the
   * stream will be buffered at all.
   * If buf is NULL then make _lbfsize 0 to force the buffer
   * to be flushed and hence malloced on first use
   */

  switch (mode)
    {
    case _IOLBF:
      fp->_flags |= __SLBF;
      fp->_lbfsize = buf ? -size : 0;
      /* FALLTHROUGH */

    case _IOFBF:
      /* no flag */
      reent->__cleanup = _cleanup_r;
      fp->_bf._base = fp->_p = (unsigned char *) buf;
      fp->_bf._size = size;
      break;
    }

  /*
   * Patch up write count if necessary.
   */

  if (fp->_flags & __SWR)
    fp->_w = fp->_flags & (__SLBF | __SNBF) ? 0 : size;

  _newlib_flockfile_end (fp);
  return 0;
}

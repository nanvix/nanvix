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
#include "fvwrite.h"
#include "local.h"

#ifdef __IMPL_UNLOCKED__
#define _fputs_r _fputs_unlocked_r
#define fputs fputs_unlocked
#endif

/*
 * Write the given string to the given file.
 */
int _fputs_r(struct _reent * ptr, char const *restrict s, FILE *restrict fp)
{
#ifdef _FVWRITE_IN_STREAMIO
  int result;
  struct __suio uio;
  struct __siov iov;

  iov.iov_base = s;
  iov.iov_len = uio.uio_resid = strlen (s);
  uio.uio_iov = &iov;
  uio.uio_iovcnt = 1;

  CHECK_INIT(ptr, fp);

  _newlib_flockfile_start (fp);
  ORIENT (fp, -1);
  result = __sfvwrite_r (ptr, fp, &uio);
  _newlib_flockfile_end (fp);
  return result;
#else
  _CONST char *p = s;

  CHECK_INIT(ptr, fp);

  _newlib_flockfile_start (fp);
  ORIENT (fp, -1);
  /* Make sure we can write.  */
  if (cantwrite (ptr, fp))
    goto error;

  while (*p)
    {
      if (__sputc_r (ptr, *p++, fp) == EOF)
	goto error;
    }
  _newlib_flockfile_exit (fp);
  return 0;

error:
  _newlib_flockfile_end (fp);
  return EOF;
#endif
}

#ifndef _REENT_ONLY

/**
 * @brief Puts a string on a stream.
 *
 * @details Writes the null-terminated string pointed
 * to by @p s to the stream pointed to by @p fp. The
 * terminating null byte is not written.
 *
 * @return Returns a non-negative number. Otherwise, it
 * returns EOF, sets an error indicator for the stream,
 * and sets errno to indicate the error.
 */
int fputs(char const *restrict s, FILE *restrict fp)
{
  return _fputs_r (_REENT, s, fp);
}

#endif /* !_REENT_ONLY */

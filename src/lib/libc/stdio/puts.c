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
#include <string.h>
#include "fvwrite.h"
#include "local.h"

/*
 * Write the given string to stdout, appending a newline.
 */

int _puts_r(struct _reent *ptr, const char *s)
{
#ifdef _FVWRITE_IN_STREAMIO
  int result;
  size_t c = strlen (s);
  struct __suio uio;
  struct __siov iov[2];
  FILE *fp;

  iov[0].iov_base = s;
  iov[0].iov_len = c;
  iov[1].iov_base = "\n";
  iov[1].iov_len = 1;
  uio.uio_resid = c + 1;
  uio.uio_iov = &iov[0];
  uio.uio_iovcnt = 2;

  _REENT_SMALL_CHECK_INIT (ptr);
  fp = _stdout_r (ptr);
  _newlib_flockfile_start (fp);
  ORIENT (fp, -1);
  result = (__sfvwrite_r (ptr, fp, &uio) ? EOF : '\n');
  _newlib_flockfile_end (fp);
  return result;
#else
  int result = EOF;
  const char *p = s;
  FILE *fp;
  _REENT_SMALL_CHECK_INIT (ptr);

  fp = _stdout_r (ptr);
  _newlib_flockfile_start (fp);
  ORIENT (fp, -1);
  /* Make sure we can write.  */
  if (cantwrite (ptr, fp))
    goto err;

  while (*p)
    {
      if (__sputc_r (ptr, *p++, fp) == EOF)
	goto err;
    }
  if (__sputc_r (ptr, '\n', fp) == EOF)
    goto err;

  result = '\n';

err:
  _newlib_flockfile_end (fp);
  return result;
#endif
}

#ifndef _REENT_ONLY

/**
 * @brief Puts a string on standard output.
 *
 * @details Writes the string pointed to by @p s, followed
 * by a <newline>, to the standard output stream stdout.
 * The terminating null byte is not written.
 *
 * @return Returns a non-negative number. Otherwise, returns EOF,
 * sets an error indicator for the stream, and errno is set to
 * indicate the error.
 */
int puts(char const *s)
{
  return _puts_r (_REENT, s);
}

#endif

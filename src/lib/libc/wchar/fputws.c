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

/*-
 * Copyright (c) 2002-2004 Tim J. Robbins.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

#include <_ansi.h>
#include <reent.h>
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <wchar.h>
#include "../stdio/fvwrite.h"
#include "../stdio/local.h"

int _fputws_r(struct _reent *ptr, const wchar_t *ws, FILE *fp)
{
  size_t nbytes;
  char buf[BUFSIZ];
#ifdef _FVWRITE_IN_STREAMIO
  struct __suio uio;
  struct __siov iov;

  _newlib_flockfile_start (fp);
  ORIENT (fp, 1);
  if (cantwrite (ptr, fp) != 0)
    goto error;
  uio.uio_iov = &iov;
  uio.uio_iovcnt = 1;
  iov.iov_base = buf;
  do
    {
      nbytes = _wcsrtombs_r(ptr, buf, &ws, sizeof (buf), &fp->_mbstate);
      if (nbytes == (size_t) -1)
	goto error;
      iov.iov_len = uio.uio_resid = nbytes;
      if (__sfvwrite_r(ptr, fp, &uio) != 0)
	goto error;
    }
  while (ws != NULL);
  _newlib_flockfile_exit (fp);
  return (0);

error:
  _newlib_flockfile_end (fp);
  return (-1);
#else
  _newlib_flockfile_start (fp);
  ORIENT (fp, 1);
  if (cantwrite (ptr, fp) != 0)
    goto error;

  do
    {
      size_t i = 0;
      nbytes = _wcsrtombs_r (ptr, buf, &ws, sizeof (buf), &fp->_mbstate);
      if (nbytes == (size_t) -1)
	goto error;
      while (i < nbytes)
        {
	  if (__sputc_r (ptr, buf[i], fp) == EOF)
	    goto error;
	  i++;
        }
    }
  while (ws != NULL);
  _newlib_flockfile_exit (fp);
  return (0);

error:
  _newlib_flockfile_end (fp);
  return (-1);
#endif
}

/**
 * @brief Puts a wide-character string on a stream.
 *
 * @details Writes a character string corresponding to the
 * (null-terminated) wide-character string pointed to by @p ws
 * to the stream pointed to by @p fp.
 *
 * @return Returns a non-negative number. Otherwise, it returns -1,
 * set an error indicator for the stream, and set errno to indicate
 * the error. 
 */
int fputws(const wchar_t *restrict ws, FILE *restrict fp)
{
  struct _reent *reent = _REENT;

  CHECK_INIT (reent, fp);
  return _fputws_r (reent, ws, fp);
}

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
#include <stdio.h>
#include <string.h>
#include <wchar.h>
#include "../stdio/local.h"

wchar_t *_fgetws_r(struct _reent *ptr, wchar_t * ws, int n, FILE * fp)
{
  wchar_t *wsp;
  size_t nconv;
  const char *src;
  unsigned char *nl;

  _newlib_flockfile_start (fp);
  ORIENT (fp, 1);

  if (n <= 0)
    {
      errno = EINVAL;
      goto error;
    }

  if (fp->_r <= 0 && __srefill_r (ptr, fp))
    /* EOF */
    goto error;
  wsp = ws;
  do
    {
      src = (char *) fp->_p;
      nl = memchr (fp->_p, '\n', fp->_r);
      nconv = _mbsnrtowcs_r (ptr, wsp, &src,
			     /* Read all bytes up to the next NL, or up to the
				end of the buffer if there is no NL. */
			     nl != NULL ? (nl - fp->_p + 1) : fp->_r,
			     /* But never more than n - 1 wide chars. */
			     n - 1,
			     &fp->_mbstate);
      if (nconv == (size_t) -1)
	/* Conversion error */
	goto error;
      if (src == NULL)
	{
	  /*
	   * We hit a null byte. Increment the character count,
	   * since mbsnrtowcs()'s return value doesn't include
	   * the terminating null, then resume conversion
	   * after the null.
	   */
	  nconv++;
	  src = memchr (fp->_p, '\0', fp->_r);
	  src++;
	}
      fp->_r -= (unsigned char *) src - fp->_p;
      fp->_p = (unsigned char *) src;
      n -= nconv;
      wsp += nconv;
    }
  while (wsp[-1] != L'\n' && n > 1 && (fp->_r > 0
	 || __srefill_r (ptr, fp) == 0));
  if (wsp == ws)
    /* EOF */
    goto error;
  if (!mbsinit (&fp->_mbstate))
    /* Incomplete character */
    goto error;
  *wsp++ = L'\0';
  _newlib_flockfile_exit (fp);
  return ws;

error:
  _newlib_flockfile_end (fp);
  return NULL;
}

/**
 * @brief Gets a wide-character string from a stream.
 *
 * @details Reads characters from the @p fp, convert these to the
 * corresponding wide-character codes, place them in the wchar_t array
 * pointed to by @p ws, until @p n-1 characters are read, or a <newline>
 * is read, converted, and transferred to @p ws, or an end-of-file condition
 * is encountered.
 *
 * @return Returns ws.
 */
wchar_t * fgetws(wchar_t *restrict ws, int n, FILE *restrict fp)
{
  struct _reent *reent = _REENT;

  CHECK_INIT (reent, fp);
  return _fgetws_r (reent, ws, n, fp);
}

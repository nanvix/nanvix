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
#include <stdlib.h>
#include <wchar.h>
#include "../stdio/local.h"

wint_t __fgetwc(struct _reent *ptr, register FILE *fp)
{
  wchar_t wc;
  size_t nconv;

  if (fp->_r <= 0 && __srefill_r (ptr, fp))
    return (WEOF);
  if (MB_CUR_MAX == 1)
    {
      /* Fast path for single-byte encodings. */
      wc = *fp->_p++;
      fp->_r--;
      return (wc);
    }
  do
    {
      nconv = _mbrtowc_r (ptr, &wc, (char *) fp->_p, fp->_r, &fp->_mbstate);
      if (nconv == (size_t)-1)
	break;
      else if (nconv == (size_t)-2)
	continue;
      else if (nconv == 0)
	{
	  /*
	   * Assume that the only valid representation of
	   * the null wide character is a single null byte.
	   */
	  fp->_p++;
	  fp->_r--;
	  return (L'\0');
	}
      else
        {
	  fp->_p += nconv;
	  fp->_r -= nconv;
	  return (wc);
	}
    }
  while (__srefill_r(ptr, fp) == 0);
  fp->_flags |= __SERR;
  errno = EILSEQ;
  return (WEOF);
}

wint_t _fgetwc_r(struct _reent *ptr, register FILE *fp)
{
  wint_t r;

  _newlib_flockfile_start (fp);
  ORIENT(fp, 1);
  r = __fgetwc (ptr, fp);
  _newlib_flockfile_end (fp);
  return r;
}

/**
 * @brief Gets a wide-character code from a stream.
 *
 * @details Obtains the next character (if present) from the input stream
 * pointed to by @p fp, convert that to the corresponding wide-character
 * code, and advance the associated file position indicator for the stream
 * (if defined).
 *
 * @return Returns the wide-character code of the character read from the 
 * input stream pointed to by stream converted to a type wint_t.
 */
wint_t fgetwc(FILE *fp)
{
  struct _reent *reent = _REENT;

  CHECK_INIT(reent, fp);
  return _fgetwc_r (reent, fp);
}

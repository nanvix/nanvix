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
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <wchar.h>
#include "../stdio/local.h"

wint_t __fputwc(struct _reent *ptr, wchar_t wc, FILE *fp)
{
  char buf[MB_LEN_MAX];
  size_t i, len;

  if (MB_CUR_MAX == 1 && wc > 0 && wc <= UCHAR_MAX)
    {
      /*
       * Assume single-byte locale with no special encoding.
       * A more careful test would be to check
       * _CurrentRuneLocale->encoding.
       */
      *buf = (unsigned char)wc;
      len = 1;
    }
  else
    {
      if ((len = _wcrtomb_r (ptr, buf, wc, &fp->_mbstate)) == (size_t) -1)
	{
	  fp->_flags |= __SERR;
	  return WEOF;
	}
    }

  for (i = 0; i < len; i++)
    if (__sputc_r (ptr, (unsigned char) buf[i], fp) == EOF)
      return WEOF;

  return (wint_t) wc;
}

wint_t _fputwc_r(struct _reent *ptr, wchar_t wc, FILE *fp)
{
  wint_t r;

  _newlib_flockfile_start (fp);
  ORIENT(fp, 1);
  r = __fputwc(ptr, wc, fp);
  _newlib_flockfile_end (fp);
  return r;
}

/**
 * @brief Puts a wide-character code on a stream.
 *
 * @details Writes the character corresponding to the wide-character
 * code @p wc to the output stream pointed to by @p fp, at the position
 * indicated by the associated file-position indicator for the stream
 * (if defined), and advances the indicator appropriately. If the file
 * cannot support positioning requests, or if the stream was opened with
 * append mode, the character is appended to the output stream. If an error
 * occurs while writing the character, the shift state of the output file is
 * left in an undefined state.
 *
 * @return Returns wc. Otherwise, it returns WEOF, the error indicator for the
 * stream is set, and errno is set to indicate the error.
 */
wint_t fputwc(wchar_t wc, FILE *fp)
{
  struct _reent *reent = _REENT;

  CHECK_INIT(reent, fp);
  return _fputwc_r (reent, wc, fp);
}

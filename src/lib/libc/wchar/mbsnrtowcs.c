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
 * Copyright (c) 1994-2009  Red Hat, Inc. All rights reserved.
 *
 * This copyrighted material is made available to anyone wishing to use,
 * modify, copy, or redistribute it subject to the terms and conditions
 * of the BSD License.   This program is distributed in the hope that
 * it will be useful, but WITHOUT ANY WARRANTY expressed or implied,
 * including the implied warranties of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE.  A copy of this license is available at 
 * http://www.opensource.org/licenses. Any Red Hat trademarks that are
 * incorporated in the source code or documentation are not subject to
 * the BSD License and may only be used or replicated with the express
 * permission of Red Hat, Inc.
 */

/*
 * Copyright (c) 1981-2000 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without modification,
 * are permitted provided that the following conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright notice, 
 *       this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright notice,
 *       this list of conditions and the following disclaimer in the documentation
 *       and/or other materials provided with the distribution.
 *     * Neither the name of the University nor the names of its contributors 
 *       may be used to endorse or promote products derived from this software 
 *       without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT,
 * INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 */

#include <reent.h>
#include <newlib.h>
#include <wchar.h>
#include <stdlib.h>
#include <stdio.h>
#include <errno.h>

size_t _mbsnrtowcs_r(struct _reent *r, wchar_t *dst, const char **src, size_t nms,
	size_t len, mbstate_t *ps)
{
  wchar_t *ptr = dst;
  const char *tmp_src;
  size_t max;
  size_t count = 0;
  int bytes;

#ifdef _MB_CAPABLE
  if (ps == NULL)
    {
      _REENT_CHECK_MISC(r);
      ps = &(_REENT_MBSRTOWCS_STATE(r));
    }
#endif

  if (dst == NULL)
    {
      /* Ignore original len value and do not alter src pointer if the
         dst pointer is NULL.  */
      len = (size_t)-1;
      tmp_src = *src;
      src = &tmp_src;
    }      
  
  max = len;
  while (len > 0)
    {
      bytes = _mbrtowc_r (r, ptr, *src, nms, ps);
      if (bytes > 0)
	{
	  *src += bytes;
	  nms -= bytes;
	  ++count;
	  ptr = (dst == NULL) ? NULL : ptr + 1;
	  --len;
	}
      else if (bytes == -2)
	{
	  *src += nms;
	  return count;
	}
      else if (bytes == 0)
	{
	  *src = NULL;
	  return count;
	}
      else
	{
	  ps->__count = 0;
	  r->_errno = EILSEQ;
	  return (size_t)-1;
	}
    }

  return (size_t)max;
}

#ifndef _REENT_ONLY

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

/**
 * @brief Converts a character string to a wide-character string (restartable).
 *
 * @details Converts a sequence of characters, beginning in the conversion state
 * described by the object pointed to by @p ps, from the array indirectly pointed
 * to by @p src into a sequence of corresponding wide characters. If @p dst is not
 * a null pointer, the converted characters is stored into the array pointed to by 
 * @p dst. Conversion continues up to and including a terminating null character, 
 * which is also be stored.
 *
 * @return If the input conversion encounters a sequence of bytes that do not form
 * a valid character, an encoding error occurs. In this case, these functions stores
 * the value of the macro [EILSEQ] in errno and returns (size_t)-1; the conversion 
 * state is undefined. Otherwise, these functions returns the number of characters
 * successfully converted, not including the terminating null (if any).
 */
size_t mbsnrtowcs(wchar_t *restrict dst, const char **restrict src, size_t nms,
	size_t len, mbstate_t *restrict ps)
{
  return _mbsnrtowcs_r (_REENT, dst, src, nms, len, ps);
}

#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
#endif /* !_REENT_ONLY */

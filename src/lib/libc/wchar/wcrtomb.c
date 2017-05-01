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
#include "../stdlib/local.h"

size_t _wcrtomb_r(struct _reent *ptr, char *s, wchar_t wc, mbstate_t *ps)
{
  int retval = 0;
  char buf[10];

#ifdef _MB_CAPABLE
  if (ps == NULL)
    {
      _REENT_CHECK_MISC(ptr);
      ps = &(_REENT_WCRTOMB_STATE(ptr));
    }
#endif

  if (s == NULL)
    retval = __wctomb (ptr, buf, L'\0', __locale_charset (), ps);
  else
    retval = __wctomb (ptr, s, wc, __locale_charset (), ps);

  if (retval == -1)
    {
      ps->__count = 0;
      ptr->_errno = EILSEQ;
      return (size_t)(-1);
    }
  else
    return (size_t)retval;
}

#ifndef _REENT_ONLY

/**
 * @brief Converts a wide-character code to a character (restartable);
 *
 * @details The main case for this function is when @p s is not NULL
 * and @p wc is not a null wide character (L'\0').  In this case, the
 * function converts the wide character @p wc to its multibyte representation
 * and stores it at the beginning of the character array pointed to by @p s.
 *
 * It updates the shift state @p *ps, and returns the length of said multibyte
 * representation, that is, the number of bytes written at @p s.

 * A different case is when @p s is not NULL, but @p wc is a null wide character
 * (L'\0').  In this case the function stores at the character array pointed to
 * by @p s the shift sequence needed to bring @p *ps back to the initial state,
 * followed by a '\0' byte. It updates the shift state @p *ps (i.e., brings it
 * into the initial state), and returns the length of the shift sequence plus one,
 * that is, the number of bytes written at s.

 * A third case is when @p s is NULL. In this case @p wc is ignored, and the function 
 * effectively returns wcrtomb(buf, L'\0', ps) where buf is an internal anonymous
 * buffer. In all of the above cases, if @p ps is a NULL pointer, a static anonymous
 * state known only to the function is used instead.
 *
 * @return Returns the number of bytes that have been or would have been written to
 * the byte array at @p s. If @p wc can not be represented as a multibyte sequence
 * (according to the current locale), (size_t)-1 is returned, and errno set to EILSEQ.
 */
size_t wcrtomb(char *restrict s, wchar_t wc, mbstate_t *restrict ps)
{
#if defined(PREFER_SIZE_OVER_SPEED) || defined(__OPTIMIZE_SIZE__)
  return _wcrtomb_r (_REENT, s, wc, ps);
#else
  int retval = 0;
  struct _reent *reent = _REENT;
  char buf[10];

#ifdef _MB_CAPABLE
  if (ps == NULL)
    {
      _REENT_CHECK_MISC(reent);
      ps = &(_REENT_WCRTOMB_STATE(reent));
    }
#endif

  if (s == NULL)
    retval = __wctomb (reent, buf, L'\0', __locale_charset (), ps);
  else
    retval = __wctomb (reent, s, wc, __locale_charset (), ps);

  if (retval == -1)
    {
      ps->__count = 0;
      reent->_errno = EILSEQ;
      return (size_t)(-1);
    }
  else
    return (size_t)retval;
#endif /* not PREFER_SIZE_OVER_SPEED */
}

#endif /* !_REENT_ONLY */

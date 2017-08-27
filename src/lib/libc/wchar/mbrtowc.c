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
#include <string.h>
#include "../stdlib/local.h"

size_t _mbrtowc_r(struct _reent *ptr, wchar_t *pwc, const char *s, size_t n,
  mbstate_t *ps)
{
  int retval = 0;

#ifdef _MB_CAPABLE
  if (ps == NULL)
    {
      _REENT_CHECK_MISC(ptr);
      ps = &(_REENT_MBRTOWC_STATE(ptr));
    }
#endif

  if (s == NULL)
    retval = __mbtowc (ptr, NULL, "", 1, __locale_charset (), ps);
  else
    retval = __mbtowc (ptr, pwc, s, n, __locale_charset (), ps);

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
 * @brief Converts a character to a wide-character code (restartable).
 *
 * @details If @p s is not a null pointer, the function inspects at most
 * @p n bytes beginning at the byte pointed to by @p s to determine the
 * number of bytes needed to complete the next character (including any
 * shift sequences). If the function determines that the next character
 * is completed, it determines the value of the corresponding wide character
 * and then, if @p pwc is not a null pointer, stores that value in the object
 * pointed to by @p pwc. If the corresponding wide character is the null wide 
 * character, the resulting state described is the initial conversion state.
 *
 * If ps is a null pointer, the function use its own internal mbstate_t object,
 * which is initialized at program start-up to the initial conversion state. 
 * Otherwise, the mbstate_t object pointed to by @p ps is used to completely 
 * describe the current conversion state of the associated character sequence.
 *
 * @return Returns 0 if the next @p n or fewer bytes complete the character that
 * corresponds to the null wide character (which is the value stored). between 1
 * and @p n inclusive. If the next @p n or fewer bytes complete a valid character
 * (which is the value stored); the value returned is the number of bytes that 
 * complete the character. Returns (size_t)-2 if the next @p n bytes contribute to
 * an incomplete but potentially valid character, and all @p n bytes have been 
 * processed (no value is stored). When @p n has at least the value of the
 * {MB_CUR_MAX} macro, this case can only occur if s points at a sequence of redundant
 * shift sequences (for implementations with state-dependent encodings). Returns 
 * (size_t)-1 if an encoding error occurs, in which case the next @p n or fewer bytes
 * do not contribute to a complete and valid character (no value is stored).
 * In this case,[EILSEQ] is stored in errno and the conversion state is undefined.
 */
size_t mbrtowc(wchar_t *restrict pwc, const char *restrict s, size_t n,
  mbstate_t *restrict ps)
{
#if defined(PREFER_SIZE_OVER_SPEED) || defined(__OPTIMIZE_SIZE__)
  return _mbrtowc_r (_REENT, pwc, s, n, ps);
#else
  int retval = 0;
  struct _reent *reent = _REENT;

#ifdef _MB_CAPABLE
  if (ps == NULL)
    {
      _REENT_CHECK_MISC(reent);
      ps = &(_REENT_MBRTOWC_STATE(reent));
    }
#endif

  if (s == NULL)
    retval = __mbtowc (reent, NULL, "", 1, __locale_charset (), ps);
  else
    retval = __mbtowc (reent, pwc, s, n, __locale_charset (), ps);

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

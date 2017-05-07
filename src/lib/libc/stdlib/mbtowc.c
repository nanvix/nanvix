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

#ifndef _REENT_ONLY

#include <newlib.h>
#include <stdlib.h>
#include <wchar.h>
#include "local.h"

/**
 * @brief Converts a character to a wide-character code.
 *
 * @details The function determines the number of bytes
 * that constitute the character pointed to by @p s. If
 * the character is valid and @p pwc is not a null pointer,
 * mbtowc() stores the wide-character code in the object pointed
 * to by @p pwc.
 *
 * @return If @p s is a null pointer, mbtowc() returns a non-zero
 * or 0 value, if character encodings, respectively, do or do not
 * have state-dependent encodings. If @p s is not a null pointer,
 * mbtowc() either returns 0 (if @p s points to the null byte), or
 * returns the number of bytes that constitute the converted character
 * (if the next @p n or fewer bytes form a valid character), or returns
 * -1 (if they do not form a valid character).
 */
int mbtowc(wchar_t *restrict pwc, const char *restrict s, size_t n)
{
#ifdef _MB_CAPABLE
  int retval = 0;
  struct _reent *reent = _REENT;
  mbstate_t *ps;

  _REENT_CHECK_MISC(reent);
  ps = &(_REENT_MBTOWC_STATE(reent));
  
  retval = __mbtowc (reent, pwc, s, n, __locale_charset (), ps);
  
  if (retval < 0)
    {
      ps->__count = 0;
      return -1;
    }
  return retval;
#else /* not _MB_CAPABLE */
  if (s == NULL)
    return 0;
  if (n == 0)
    return -1;
  if (pwc)
    *pwc = (wchar_t) *s;
  return (*s != '\0');
#endif /* not _MB_CAPABLE */
}

#endif /* !_REENT_ONLY */

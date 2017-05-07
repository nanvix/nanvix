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

#include <_ansi.h>
#include <wchar.h>
#include "../string/local.h"

#ifdef _XOPEN_SOURCE

/**
 * @brief Number of column positions of a wide-character string.
 *
 * @details Determines the number of column positions required for
 * @p n wide-character codes (or fewer than @p n wide-character codes if
 * a null wide-character code is encountered before @p n wide-character
 * codes are exhausted) in the string pointed to by @p pwcs.
 *
 * @return Returns 0 (if pwcs points to a null wide-character code), or
 * returns the number of column positions to be occupied by the wide-character
 * string pointed to by @p pwcs, or returns -1 (if any of the first @p n 
 * wide-character codes in the wide-character string pointed to by pwcs is not
 * a printable wide-character code).
 */
int wcswidth(const wchar_t *pwcs, size_t n)
{
  int w, len = 0;
  if (!pwcs || n == 0)
    return 0;
  do {
    wint_t wi = *pwcs;

#ifdef _MB_CAPABLE
  wi = _jp2uc (wi);
  /* First half of a surrogate pair? */
  if (sizeof (wchar_t) == 2 && wi >= 0xd800 && wi <= 0xdbff)
    {
      wint_t wi2;

      /* Extract second half and check for validity. */
      if (--n == 0 || (wi2 = _jp2uc (*++pwcs)) < 0xdc00 || wi2 > 0xdfff)
	return -1;
      /* Compute actual unicode value to use in call to __wcwidth. */
      wi = (((wi & 0x3ff) << 10) | (wi2 & 0x3ff)) + 0x10000;
    }
#endif /* _MB_CAPABLE */
    if ((w = __wcwidth (wi)) < 0)
      return -1;
    len += w;
  } while (*pwcs++ && --n > 0);
  return len;
}

#endif /* _XOPEN_SOURCE */

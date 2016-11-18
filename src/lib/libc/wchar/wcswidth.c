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

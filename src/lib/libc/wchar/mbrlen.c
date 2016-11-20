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

#include <reent.h>
#include <newlib.h>
#include <wchar.h>
#include <stdlib.h>
#include <stdio.h>
#include <errno.h>

/**
 * @brief Gets number of bytes in a character (restartable).
 *
 * @details Determines the number of bytes constituting the
 * character pointed to by @p s.
 *
 * @return Returns the number of bytes parsed from the multibyte
 * sequence starting at @p s, if a non-null wide character was recognized.
 * It returns 0, if a null wide character was recognized. It returns (size_t)-1
 * and sets errno to EILSEQ, if an invalid multibyte sequence was encountered.
 * It returns (size_t)-2 if it couldn't parse a complete multibyte character,
 * meaning that @p n should be increased.
 */
size_t mbrlen(const char *restrict s, size_t n, mbstate_t *restrict ps)
{
#ifdef _MB_CAPABLE
  if (ps == NULL)
    {
      struct _reent *reent = _REENT;

      _REENT_CHECK_MISC(reent);
      ps = &(_REENT_MBRLEN_STATE(reent));
    }
#endif

  return mbrtowc(NULL, s, n, ps);
}

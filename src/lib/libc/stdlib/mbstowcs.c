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

#ifndef _REENT_ONLY

#include <newlib.h>
#include <stdlib.h>
#include <wchar.h>

/**
 * @brief Converts a character string to a wide-character string.
 *
 * @details Converts a sequence of characters that begins in the
 * initial shift state from the array pointed to by @p s into a
 * sequence of corresponding wide-character codes and stores not
 * more than @p n wide-character codes into the array pointed to by
 * @p pwcs.
 *
 * @return Returns (size_t)-1 if an invalid character is encountered.
 * Otherwise, the function returns the number of the array elements
 * modified not including a terminating 0 code, if any.
 */
size_t mbstowcs(wchar_t *restrict pwcs, const char *restrict s, size_t n)
{
#ifdef _MB_CAPABLE
  mbstate_t state;
  state.__count = 0;
  
  return _mbstowcs_r (_REENT, pwcs, s, n, &state);
#else /* not _MB_CAPABLE */
  
  int count = 0;
  
  if (n != 0) {
    do {
      if ((*pwcs++ = (wchar_t) *s++) == 0)
	break;
      count++;
    } while (--n != 0);
  }
  
  return count;
#endif /* not _MB_CAPABLE */
}

#endif /* !_REENT_ONLY */

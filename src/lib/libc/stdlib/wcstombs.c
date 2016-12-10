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
 * @brief Converts a wide-character string to a character string.
 *
 * @details Converts the sequence of wide-character codes that
 * are in the array pointed to by @p pwcs into a sequence of 
 * characters that begins in the initial shift state and store
 * these characters into the array pointed to by @p s, stopping
 * if a character would exceed the limit of @p n total bytes or
 * if a null byte is stored.
 *
 * @return If a wide-character code is encountered that does not
 * correspond to a valid character (of one or more bytes each),
 * wcstombs() returns (size_t)-1. Otherwise, wcstombs() returns
 * the number of bytes stored in the character array, not including
 * any terminating null byte.
 */
size_t wcstombs(char *restrict s, const wchar_t *restrict pwcs, size_t n)
{
#ifdef _MB_CAPABLE
  mbstate_t state;
  state.__count = 0;
  
  return _wcstombs_r (_REENT, s, pwcs, n, &state);
#else /* not _MB_CAPABLE */
  int count = 0;
  
  if (n != 0) {
    do {
      if ((*s++ = (char) *pwcs++) == 0)
	break;
      count++;
    } while (--n != 0);
  }
  
  return count;
#endif /* not _MB_CAPABLE */
}

#endif /* !_REENT_ONLY */

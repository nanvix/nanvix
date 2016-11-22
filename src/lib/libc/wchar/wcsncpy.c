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

/**
 * @brief Copies a fixed-size wide-character string, returning a pointer to 
 * its end.
 *
 * @details Copies not more than n wide-character codes (wide-character
 * codes that follow a null wide-character code are not copied) from the
 * array pointed to by @p s2 to the array pointed to by @p s1.
 *
 * @return If any null wide-character codes were written into the destination,
 * the function returns the address of the first such null wide-character code.
 * Otherwise, it returns @p &s1[n].
 */
wchar_t *wcsncpy(wchar_t *restrict s1, const wchar_t *restrict s2, size_t n)
{
  wchar_t *dscan=s1;

  while(n > 0)
    {
      --n;
      if((*dscan++ = *s2++) == L'\0')  break;
    }
  while(n-- > 0)  *dscan++ = L'\0';

  return s1;
}

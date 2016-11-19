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

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

/**
 * @brief Copies a fixed-size wide-character string, returning a pointer to its end.
 *
 * @details Copies not more than @p count wide-character codes (wide-character
 * codes that follow a null wide-character code are not copied) from the array
 * pointed to by @p src to the array pointed to by @p dst. If copying takes place
 * between objects that overlap, the behavior is undefined.
 *
 * @return Returns the address of the first such null wide-character code. Otherwise,
 * it shall return @p &dst[n].
 */
wchar_t *wcpncpy(wchar_t *restrict dst, wchar_t *restrict src, size_t count)
{
  wchar_t *ret = NULL;

  while (count > 0)
    {
      --count;
      if ((*dst++ = *src++) == L'\0')
	{
	  ret = dst - 1;
	  break;
	}
    }
  while (count-- > 0)
    *dst++ = L'\0';

  return ret ? ret : dst;
}

#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */

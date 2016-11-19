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

#include <wchar.h>
#include <wctype.h>

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

/**
 * @brief Case-insensitive wide-character string comparison.
 *
 * @details Compares, while ignoring differences in case, the
 * wide-character string pointed to by @p s1 to the wide-character
 * string pointed to by @p s2.
 *
 * @return Returns an integer greater than, equal to, or less than 0
 * if the wide-character string pointed to by @p s1 is, ignoring case,
 * greater than, equal to, or less than the wide-character string pointed
 * to by @p s2, respectively.
 */
int wcscasecmp(const wchar_t *s1, const wchar_t *s2)
{
  while (*s1 != '\0' && towlower(*s1) == towlower(*s2))
    {
      s1++;
      s2++;
    }

  return towlower(*s1) - towlower(*s2);
}

#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */

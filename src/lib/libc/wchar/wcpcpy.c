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
 * @brief Copies a wide-character string, returning a pointer to its end.
 *
 * @details Copies the wide-character string pointed to by @p s2 (including
 * the terminating null wide-character code) into the array pointed to by @p s1.
 *
 * @return Returns a pointer to the terminating null wide-character code copied
 * into the @p s1 buffer.
 */
wchar_t *wcpcpy(wchar_t *restrict s1, const wchar_t *restrict s2)
{
  while ((*s1++ = *s2++))
    ;
  return --s1;
}

#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */

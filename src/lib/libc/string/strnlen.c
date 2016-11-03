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

#undef __STRICT_ANSI__
#include <_ansi.h>
#include <string.h>

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

/**
 * @brief Gets length of fixed size string.
 *
 * @details Computes the smaller of the number of bytes in the
 * array to which @p str points, not including any terminating
 * NUL character, or the value of the @p n argument.
 *
 * @return Returns the length of @p str.
 */
size_t strnlen(const char *str, size_t n)
{
  const char *start = str;

  while (n-- > 0 && *str)
    str++;

  return str - start;
}

#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */

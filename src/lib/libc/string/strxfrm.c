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

#include <string.h>

/**
 * @brief String transformation.
 *
 * @details Transforms the C string pointed by @p s2 according
 * to the current locale and copies the first @p n characters 
 * of the transformed string to destination.
 *
 * @return Returns the length of the transformed string (not 
 * including the terminating NUL character).
 */
size_t strxfrm(char *restrict s1, const char *restrict s2, size_t n)
{
  size_t res;
  res = 0;
  while (n-- > 0)
    {
      if ((*s1++ = *s2++) != '\0')
        ++res;
      else
        return res;
    }
  while (*s2)
    {
      ++s2;
      ++res;
    }

  return res;
}

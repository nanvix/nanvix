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
 * @brief Gets the length of a complementary substring.
 *
 * @details Computes the length (in bytes) of the maximum
 * initial segment of the string pointed to by @p s1 which 
 * consists entirely of bytes not from the string pointed 
 * to by @p s2.
 *
 * @return Returns the length of the computed segment of the
 * string pointed to by @p s1.
 */
size_t strcspn(const char *s1, const char *s2)
{
  const char *s = s1;
  const char *c;

  while (*s1)
    {
      for (c = s2; *c; c++)
	{
	  if (*s1 == *c)
	    break;
	}
      if (*c)
	break;
      s1++;
    }

  return s1 - s;
}

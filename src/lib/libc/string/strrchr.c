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
 * @brief String scanning operation.
 *
 * @details Locates the last occurrence of @p i (converted
 * to a char) in the string pointed to by @p s. The terminating
 * NUL character is considered to be part of the string.
 *
 * @return Returns a pointer to the byte or a null pointer if @p i
 * does not occur in the string.
 */
char *strrchr(const char *s, int i)
{
  const char *last = NULL;

  if (i)
    {
      while ((s=strchr(s, i)))
	{
	  last = s;
	  s++;
	}
    }
  else
    {
      last = strchr(s, i);
    }
		  
  return (char *) last;
}

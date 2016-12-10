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

/*
 * bsearch.c
 * Original Author:	G. Haley
 * Rewritten by:	G. Noer
 *
 * Searches an array of nmemb members, the initial member of which is pointed
 * to by base, for a member that matches the object pointed to by key. The
 * contents of the array shall be in ascending order according to a comparison
 * function pointed to by compar. The function shall return an integer less
 * than, equal to or greater than zero if the first argument is considered to be
 * respectively less than, equal to or greater than the second. Returns a
 * pointer to the matching member of the array, or a null pointer if no match
 * is found.
 */

#include <stdlib.h>

/**
 * @brief Binary search a sorted table.
 *
 * @details Searchs an array of @p nmemb objects, the initial
 * element of which is pointed to by @p base, for an element
 * that matches the object pointed to by @p key. The size of
 * each element in the array is specified by @p size. If the
 * @p nmemb argument has the value zero, the comparison function
 * pointed to by @p compar is not called and no match is found.
 */
void *bsearch(const void *key, const void *base, size_t nmemb, size_t size,
  int (*compar)(const void *, const void *))
{
  void *current;
  size_t lower = 0;
  size_t upper = nmemb;
  size_t index;
  int result;

  if (nmemb == 0 || size == 0)
    return NULL;

  while (lower < upper)
    {
      index = (lower + upper) / 2;
      current = (void*) (((char *) base) + (index * size));

      result = compar (key, current);

      if (result < 0)
        upper = index;
      else if (result > 0)
        lower = index + 1;
      else
	return current;
    }

  return NULL;
}

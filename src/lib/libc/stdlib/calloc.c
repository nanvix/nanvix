/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

/**
 * @file
 *
 * @brief calloc() implementation.
 */

#include <stdlib.h>
#include <string.h>

/**
 * @brief Allocates a chunk of memory and cleans it.
 *
 * @param n    Number of elements to allocate.
 * @param size Size of each element.
 *
 * @returns Upon successful completion with both @n and @p size non-zero,
 *          calloc() returns a pointer to the allocated space. If either @p n
 *          or @p size is 0, then either a null pointer or a unique pointer
 *          value that can be successfully passed to free() is returned.
 *          Otherwise, it returns a null pointer and set errno to indicate the
 *          error.
 */
void *calloc(size_t n, size_t size)
{
  register char *cp;

  n *= size;

  cp = malloc(n);
  if (cp == NULL)
    return (NULL);

  memset(cp, 0, n);
  return (cp);
}

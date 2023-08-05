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
 * @brief memcpy() implementation.
 */

#include <sys/types.h>

/**
 * @brief Copies bytes in memory.
 *
 * @param s1 Pointer to target object.
 * @param s2 Pointer to source object.
 * @param n  Number of bytes to copy.
 *
 * @returns @p s1 is returned.
 *
 * @note If copying takes place between objects that overlap, the behavior is
 *       undefined.
 */
void *memcpy(void *restrict s1, const void *restrict s2, size_t n)
{
    char *p1;
    const char* p2;

    p1 = s1;
    p2 = s2;

    while (n-- > 0)
    	*p1++ = *p2++;

    return (p1);
}

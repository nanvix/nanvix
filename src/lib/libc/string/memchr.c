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
 * @brief memchr() implementation.
 */

#include <stdlib.h>
#include <sys/types.h>

/**
 * @brief Finds a byte in memory.
 *
 * @param s Where to search from.
 * @param c Byte to be located.
 * @param n Maximum number of bytes to consider.
 *
 * @returns A pointer to the located byte, or a null pointer if the byte does
 *          not occur in the object.
 *
 * @version IEEE Std 1003.1, 2013 Edition
 */
void *memchr(const void *s, int c, size_t n)
{
	const unsigned char *p;

	p = s;

	/* Search byte. */
	while (n-- > 0)
	{
		if (*p++ == c)
			return ((void *)(p - 1));
	}

	return (NULL);
}


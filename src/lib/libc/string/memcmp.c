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
 * @brief memcmp() implementation.
 */

#include <sys/types.h>

/**
 * @brief Compares bytes in memory.
 *
 * @param s1 Pointer to object 1.
 * @param s2 Pointer to object 2.
 * @param n  Number of bytes to compare.
 *
 * @returns An integer greater than, equal to or less than 0, if the object
 *          pointed to by @p s1 is greater than, equal to or less than the
 *          object pointed to by @p s2 respectively.
 *
 * @version IEEE Std 1003.1, 2013 Edition
 */
int memcmp(const void *s1, const void *s2, size_t n)
{
	const unsigned char *p1;
	const unsigned char *p2;

	p1 = s1;
	p2 = s2;

	while (n-- > 0)
	{
		if (*p1++ != *p2++)
			return (*--p1 - *--p2);
	}

	return (0);
}


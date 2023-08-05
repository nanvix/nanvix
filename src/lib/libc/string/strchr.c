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
 * @brief strchr() implementation.
 */

#include <stdlib.h>
#include <sys/types.h>

/**
 * @brief Finds a byte in string.
 *
 * @param s String to search.
 * @param c Character to search.
 *
 * @returns A pointer to the byte, or a null pointer if the byte was not found.
 *
 * @version IEEE Std 1003.1, 2013 Edition
 */
char *strchr(const char *s, int c)
{
	/* Scan string. */
	while (*s != '\0')
	{
		/* Found. */
		if (*s == c)
			return ((char *) s);

		s++;
	}

	return ((c == '\0') ? (char *) s : NULL);
}

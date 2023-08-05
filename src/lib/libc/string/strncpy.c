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
 * @brief strncpy() implementation.
 */

#include <sys/types.h>

/**
 * @brief Copies part of a string.
 *
 * @param s1 Pointer to target string.
 * @param s2 Pointer to source string.
 * @param n  Number of characters to copy.
 *
 * @param @p s1 is returned.
 *
 * @version IEEE Std 1003.1, 2013 Edition
 */
char *strncpy(char *restrict s1, const char *restrict s2, size_t n)
{
	char *p1;       /* Indexes s1. */
	const char *p2; /* Indexes s2.  */

	p1 = s1;
	p2 = s2;

	/* Copy string. */
	while (n > 0)
	{
		if (*p2 == '\0')
			break;

		*p1++ = *p2++;
		n--;
	}

	/* Fill with null bytes. */
	while (n > 0)
	{
		 *p1++ = '\0';
		 n--;
	}

	return (s1);
}

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
 * @brief strncmp() implementation.
 */

#include <sys/types.h>

/**
 * @brief Compares part of two strings.
 *
 * @param s1 Pointer to first string.
 * @param s2 Pointer to second string.
 * @param n  Number of characters to consider.
 *
 * @returns An integer greater than, equal to or less than 0, if the possibly
 *          null-terminated array pointed to by @p s1 is greater than, equal to
 *          or less than the possibly null-terminated array pointed to by @p s2
 *          respectively.
 *
 * @version IEEE Std 1003.1, 2013 Edition
 */
int strncmp(const char *restrict s1, const char *restrict s2, size_t n)
{
	/* Compare strings. */
	while (n > 0)
	{
		/* Strings differ. */
		if (*s1 != *s2)
			return ((*(unsigned char *) s1 - *(unsigned char *) s2));

		/* End of string. */
		else if (*s1 == '\0')
			return (0);

		s1++;
		s2++;
		n--;
	}

	return (0);
}

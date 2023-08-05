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
 * @brief strcmp() implementation.
 */

/**
 * @brief Compares two strings.
 *
 * @param s1 Pointer to first string.
 * @param s2 Pointer to second string.
 *
 * @returns An integer greater than, equal to or less than 0, if the string
 *          pointed to by @p s1 is greater than, equal to or less than the
 *          string pointed to by @p s2 respectively.
 *
 * @version IEEE Std 1003.1, 2013 Edition
 */
int strcmp(const char *s1, const char *s2)
{
	/* Compare strings. */
	while (*s1 == *s2)
	{
		/* End of string. */
		if (*s1 == '\0')
			return (0);

		s1++;
		s2++;
	}

	return ((*(unsigned char *) s1 - *(unsigned char *) s2));
}

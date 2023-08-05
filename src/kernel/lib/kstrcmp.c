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

#include <nanvix/const.h>

/**
 * @brief Compares two strings.
 *
 * @param str1 String one.
 * @param str2 String two.
 *
 * @returns Zero if the strings are equal, and non-zero otherwise.
 */
PUBLIC int kstrcmp(const char *str1, const char *str2)
{
	/* Compare strings. */
	while (*str1 == *str2)
	{
		/* End of string. */
		if (*str1 == '\0')
			return (0);

		str1++;
		str2++;
	}

	return (*str1 - *str2);
}

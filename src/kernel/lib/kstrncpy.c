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
#include <sys/types.h>

/**
 * @brief Copies part of a string.
 *
 * @param str1 Target string.
 * @param str2 Source string.
 * @param n    Number of characters to be copied.
 *
 * @returns A pointer to the target string.
 */
PUBLIC char *kstrncpy(char *str1, const char *str2, size_t n)
{
	char *p1;       /* Indexes str1. */
	const char *p2; /* Indexes str2. */

	p1 = str1;
	p2 = str2;

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

	return (str1);
}

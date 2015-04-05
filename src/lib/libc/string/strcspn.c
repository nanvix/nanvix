/*
 * Copyright(C) 2011-2015 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * @brief strcspn() implementation.
 */

#include <sys/types.h>

/**
 * @addtogroup string
 */
/**@{*/

/**
 * @brief Gets length of a complementary substring.
 * 
 * @details Computes the length of the maximum initial segment of the string
 *          pointed to by @p s1 which consists entirely of bytes not from the
 *          string pointed to by @p s2. 
 * 
 * @returns The length of the computed segment of the string pointed to by
            @p s1; no return value is reserved to indicate an error.
 */
size_t strcspn(const char *s1, const char *s2)
{
	const char *p, *spanp;
	char c, sc;

	/*
	 * Stop as soon as we find any character from s2.  Note that there
	 * must be a NULL in s2; it suffices to stop when we find that, too.
	 */
	for (p = s1; /* noop */; /* noop */)
	{
		c = *p++;
		spanp = s2;
		
		do {
			if ((sc = *spanp++) == c)
				return (p - 1 - s1);
		} while (sc != '\0');
	}
	
	/* Not reached. */
	return (0);
}

/**@}*/

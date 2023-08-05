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
 * @brief strcat() implementation.
 */

#include <sys/types.h>

/**
 * @brief Concatenates two strings.
 *
 * @param s1 Pointer to target string.
 * @param s2 Pointer to source string.
 *
 * @returns @p s1 is returned.
 *
 * @version IEEE Std 1003.1, 2013 Edition
 */
char *strcat(char *restrict s1, const char *restrict s2)
{
	char *save = s1;

	/* Eat target string. */
	for (; *s1; s1++)
		/* noop*/ ;

	/* Concatenate strings. */
	while ((*s1++ = *s2++) != '\0')
		/* noop */;

	return(save);
}

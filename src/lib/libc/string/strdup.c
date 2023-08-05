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
 * @brief strdup() implementation.
 */

#include <stdlib.h>
#include <string.h>

/**
 * @brief Duplicates a string.
 *
 * @details Returns a pointer to a new string, which is a duplicate of the
 *          string pointed to by @p s1. The returned pointer can be passed to
 *          #free(). A null pointer is returned if the new string cannot be
 *          created.
 *
 * @returns A pointer to a new string on success. Otherwise it returns a null
 *          pointer and sets #errno to indicate the error.
 *
 * @exception #ENOMEM Storage space available is insufficient.
 */
char *strdup(const char *s1)
{
	int len;
	char *copy;

	len = strlen(s1) + 1;
	if (!(copy = malloc(len)))
		return((char *)NULL);
	strcpy(copy, s1);
	return(copy);
}

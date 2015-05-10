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
 * @brief getenv() implementation.
 */

#include <unistd.h>
#include <stdlib.h>
#include <string.h>

/**
 * @brief Finds environment variable.
 * 
 * @param name   Variable name.
 * @param offset Variable offset store location.
 * 
 * @returns A pointer to a string containing the value for the specified name,
 *          upon successful completion. If the specified name cannot be found
 *          in the environment of the calling process, a null pointer is
 *          returned instead.
 */
char *findenv(const char *name, int *offset)
{
	register int length; /* Variable name length.         */
	const char *c;       /* Environment variable name.    */
	register char **p;   /* Working environment variable. */

	c = name;
	length = 0;
	while ((*c != '\0') && (*c != '='))
	{
		c++;
		length++;
	}

	/* Search for environment variable. */
	for (p = environ; *p != NULL; p++)
	{
		/* Found. */
		if (!strncmp (name, *p, length))
		{
			if (*(c = *p + length) == '=')
			{
				*offset = p - environ;
				return ((char *) (++c));
			}
		}
	}
	
	return (NULL);
}

/**
 * @brief Gets value of an environment variable.
 * 
 * @param name Variable name.
 * 
 * @returns A pointer to a string containing the value for the specified name,
 *          upon successful completion. If the specified name cannot be found
 *          in the environment of the calling process, a null pointer is
 *          returned instead.
 */
char *getenv(const char *name)
{
	int offset;
 
	return (findenv(name, &offset));
}

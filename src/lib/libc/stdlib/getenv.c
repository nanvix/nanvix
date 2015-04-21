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
 * @brief Gets value of an environment variable.
 * 
 * @details Searches the environment of the calling process for the environment
 *          variable @p name. If it exists, it returns a pointer to the value of
 *          the environment variable. If the specified environment variable
 *          cannot be found, a null pointer is returned. The application shall
 *          ensure that it does not modify the string pointed to by the getenv()
 *          function.
 * 
 *          The returned string pointer might be invalidated or the string
 *          content might be overwritten by a subsequent call to getenv(),
 *          setenv(), unsetenv(), or (if supported) putenv() but it is not
 *          affected by a call to any other function.
 * 
 * @returns Upon successful completion, getenv() returns a pointer to a string
 *          containing the value for the specified name. If the specified
 *          name cannot be found in the environment of the calling process,
 *          a null pointer is returned.
 */
char *getenv(const char *name)
{
	char **p;   /* Working environment variable. */
	int length; /* Environment variable name.    */
	
	length = strlen(name);
	
	/* Search for environment variable. */
	for (p = environ; *p != NULL; p++)
	{
		/* Found. */
		if (!strncmp(name, *p, length))
			return (*p + length + 1);
	}
	
	return (NULL);
}

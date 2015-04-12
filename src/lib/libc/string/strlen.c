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
 * @brief strlen() implementation.
 */

#include <sys/types.h>

/**
 * @addtogroup stringlib
 */
/**@{*/

/**
 * @brief Gets string length.
 * 
 * @details Computes the number of bytes in the string to which @p str points,
 *          not including the terminating null byte.
 * 
 * @returns The length of @p str; no return value is reserved to indicate an
 *          error.
 */
size_t strlen(const char * str)
{
	const char *p;
	
	/* Count the number of characters. */
	for (p = str; *p != '\0'; p++)
		/* No operation.*/;
	
	return (p - str);
}

/**@}*/

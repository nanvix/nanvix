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
 * @brief strncpy() implementation.
 */

#include <sys/types.h>

/**
 * @brief Copies part of a string.
 * 
 * @details Copies not more than @p n bytes (bytes that follow a null byte are
 *          not copied) from the array pointed to by @p s2 to the array pointed
 *          to by @p s1. If copying takes place between objects that overlap,
 *          the behavior is undefined.
 * 
 *          If the array pointed to by @p s2 is a string that is shorter than
 *          @p n bytes, null bytes are appended to the copy in the array pointed
 *          to by @p s1, until @p n bytes in all are written.
 * 
 * @returns @p s1 is returned; no return value is reserved to indicate an error. 
 */
char *strncpy(char *restrict s1, const char *restrict s2, size_t n)
{
	char *p1;       /* Indexes s1. */
	const char *p2; /* Indexes s2.  */
	
	p1 = s1;
	p2 = s2;
	
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
	
	return (s1);
}

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
 * @brief strcpy() implementation.
 */

/**
 * @brief Copies a string.
 * 
 * @param Copies the string pointed to by @p s2 (including the terminating null
 *        byte) into the array pointed to by @p s1. If copying takes place
 *        between objects that overlap, the behavior is undefined.
 * 
 * @returns s1 is returned.
 */
char *strcpy(char *s1, const char *s2)
{
	char *p;
	
	p = s1;
	
	/* Copy strings. */
	while (*s2 != '\0')
		*p++ = *s2++;
	*p = '\0';
	
	return (s1);
}

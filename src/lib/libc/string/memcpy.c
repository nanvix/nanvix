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
 * @brief memcpy() implementation.
 */

#include <sys/types.h>

/**
 * @brief Copies bytes in memory.
 * 
 * @details Copies @p n bytes from the object pointed to by @p s2 into the
 *          object pointed to by @p s1. If copying takes place between objects
 *          that overlap, the behavior is undefined. 
 * 
 * @returns @p s1 is returned.
 * 
 * @note Does not check for the overflowing of the receiving memory area. 
 */
void *memcpy(void *restrict s1, const void *restrict s2, size_t n)
{
    const char* s;
    char *d;
    
    s = s2;
    d = s1;
    
    while (n-- > 0)
    	*d++ = *s++;
 
    return (d);
}

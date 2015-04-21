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

/*
 * Copyright (c) 1990 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 * 4. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

/**
 * @file
 * 
 * @brief _Exit() implementation.
 */

#include <stddef.h>
#include <stdlib.h>

/**
 * @brief Performs a binary search on a sorted table.
 * 
 * @details Searches an array of @p nmemb objects, the initial element of which
 *          is pointed to by @p base, for an element that matches the object
 *          pointed to by @p key. The size of each element in the array is
 *          specified by @p size. If the @p nmemb argument has the value zero,
 *          the comparison function pointed to by @p compar is not called and no
 *          match is found.
 * 
 *         The comparison function pointed to by @p compar is called with two
 *         arguments that point to the key object and to an array element, in
 *         that order.
 * 
 *         The application shall ensure that the comparison function pointed to
 *         by @p compar does not alter the contents of the array. The
 *         implementation reorders elements of the array between calls to the
 *         comparison function, but it does not alter the contents of any
 *         individual element.
 * 
 *         The application shall ensure that the first argument is always a
 *         pointer to the key.
 * 
 *         When the same objects are passed more than once to the comparison
 *         function, the results shall be consistent with one another. That is,
 *         the same object always compare the same way with the key.
 * 
 *         The application shall ensure that the function returns an integer
 *         less than, equal to, or greater than 0 if the key object is
 *         considered, respectively, to be less than, to match, or to be greater
 *         than the array element. The application shall ensure that the array
 *         consists of all the elements that compare less than, all the elements
 *         that compare equal to, and all the elements that compare greater than
 *         the key object, in that order.
 *
 * @returns A pointer to a matching member of the array, or a null pointer if
 *          no match is found. If two or more members compare equal, which
 *          member is returned is unspecified.
 */
void *bsearch(const void *key, const void *base, size_t nmemb, size_t size,
		int (*compar)(const void *, const void *))
{
	void *current;
	int result;

	if (nmemb == 0 || size == 0)
		return (NULL);

	while (nmemb != 0)
	{
		current = (void *) (((char *) base) + ((nmemb / 2) * size));

		result = compar(key, current);

		if (result < 0)
			nmemb /= 2;
		else if (result > 0)
		{
			base = (void *) (((char *) current) + size);
			nmemb = (nmemb / 2) - (nmemb % 2 ? 0 : 1);
		}
		else
			return (current);
    }

  if (compar(key, base) == 0)
    return ((void *) base);

  return (NULL);
}

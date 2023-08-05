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
 * @brief qsort() implementation.
 */

#include <stddef.h>
#include <stdlib.h>
#include <string.h>

/**
 * @brief Compare function.
 */
static int (*_cmp)(const void *, const void *) = NULL;

/**
 * @brief Finds sorting pivot.
 */
static int find_pivot(void *base, int i, int j, size_t size)
{
	void *first_key;
	void *next_key;

	first_key = (void *) (((char *) base) + (i*size));
	next_key = first_key;

	for (int k = i + 1; k <= j; k++)
    {
		int res;

		next_key = (void *) (((char *) next_key) + size);
		res = (*_cmp) (next_key, first_key);

		if (res > 0)
			return (k);
		else if (res < 0)
			return (i);
	}

	return (-1);
}

/**
 * @brief Swaps two elements.
 */
static void swap(void *base, int i, int j, size_t size)
{
	static void *temp = NULL;
	static size_t max_size = 0;
	void *elem1, *elem2;

	if (size > max_size)
	{
		temp = realloc(temp, size);
		max_size = size;
	}

	elem1 = ((char *) base) + (i * size);
	elem2 = ((char *) base) + (j * size);

	memcpy (temp, elem1, size);
	memcpy (elem1, elem2, size);
	memcpy (elem2, temp, size);
}

/**
 * @brief Partitions the array.
 */
static int partition(void *base, int i, int j, int pivot_index, size_t size)
{
	int left, right;

	left = i;
	right = j;

	do
	{
		swap (base, left, right, size);

		if (pivot_index == left)
			pivot_index = right;
		else if (pivot_index == right)
			pivot_index = left;

		while (_cmp ((void *) (((char *) base) + (left * size)),
			(void *) (((char *) base) + (pivot_index * size))) < 0)
			left++;

		while (_cmp ((void *) (((char *) base) + (right * size)),
			(void *) (((char *) base) + (pivot_index * size))) >= 0)
			right--;
	} while (left <= right);

	return (left);
}

/**
 * @brief Internal qsort().
 */
static void _qsort(void *base, int i, int j, size_t size)
{
	int pivot_index, mid;

	pivot_index = find_pivot(base, i, j, size);

	if (pivot_index == -1)
		return;

	/* Sort. */
	mid = partition (base, i, j, pivot_index, size);
	_qsort (base, i, mid - 1, size);
	_qsort (base, mid, j, size);
}

/**
 * @brief Sorts a table of data.
 *
 * @param base  Array to sort.
 * @param nmemb Number of elements in the array.
 * @param size  Size of each element.
 * @param cmp   Comparison function.
 */
void qsort
(void *base, size_t nmemb, size_t size, int (*cmp)(const void *, const void *))
{
	_cmp = cmp;
	_qsort(base, 0, nmemb - 1, size);
}

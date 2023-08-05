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

#include <nanvix/mm.h>
#include <sys/types.h>
#include <stdlib.h>
#include <unistd.h>

/* Current breakpoint value. */
static void *current_brk = (void *)UHEAP_ADDR;

/*
 * Changes process's breakpoint value.
 */
void *sbrk(size_t size)
{
	void *old_brk;

	size = (((size) + (PAGE_SIZE - 1)) & ~(PAGE_SIZE - 1));

	old_brk = current_brk;
	current_brk = (void *)((unsigned)old_brk + size);
	if (brk(current_brk))
	{
		current_brk = old_brk;
		return ((void *) -1);
	}

	return (old_brk);
}

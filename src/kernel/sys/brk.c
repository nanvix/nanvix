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

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <nanvix/region.h>
#include <errno.h>
#include <nanvix/klib.h>
/*
 * Changes process' breakpoint value.
 */
PUBLIC int sys_brk(void *addr)
{
	ssize_t size;         /* Increment size.     */
	struct pregion *heap; /* Heap process region.*/

	heap = findreg(curr_proc, (addr_t)addr);

	/* Bad address. */
	if (heap != HEAP(curr_proc))
		return (-EFAULT);

	size = (addr_t)addr - (heap->start + heap->reg->size);

	/*
	 * There is no need to lock the heap region
	 * since it is a private region.
	 */

	return (growreg(curr_proc, heap, size));
}

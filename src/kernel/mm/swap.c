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

#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/klib.h>
#include <nanvix/hal.h>
#include <nanvix/mm.h>
#include "mm.h"

#if (LIVE_IMAGE != 1)

/*
 * Swapping area too small?
 */
#if (SWP_SIZE < MEMORY_SIZE)
	#error "swapping area to small"
#endif

/**
 * @brief Swap space.
 */
PRIVATE struct
{
	unsigned count[SWP_SIZE/PAGE_SIZE];         /**< Reference count. */
	uint32_t bitmap[(SWP_SIZE/PAGE_SIZE) >> 5]; /**< Bitmap.          */
} swap = {{0, }, {0, } };

/**
 * @brief Clears the swap space that is associated to a page.
 * 
 * @param pg Page to be inspected.
 */
PUBLIC void swap_clear(struct pte *pg)
{
	unsigned i;
	
	i = pg->frame;
	
	/* Free swap space. */
	if (swap.count[i] > 0)
	{
		bitmap_clear(swap.bitmap, i);
		swap.count[i]--;
	}
}

/**
 * @brief Swaps a page out to disk.
 * 
 * @param proc Process where the page is located.
 * @param addr Address of the page to be swapped out.
 * 
 * @returns Zero upon success, and non-zero otherwise.
 */
PUBLIC int swap_out(struct process *proc, addr_t addr)
{
	unsigned blk;   /* Block number in swap device.  */
	struct pte *pg; /* Page table entry.             */
	off_t off;      /* Offset in swap device.        */
	ssize_t n;      /* # bytes written.              */
	void *kpg;      /* Kernel page used for copying. */
	
	addr &= PAGE_MASK;
	pg = getpte(proc, addr);
	
	/* Get kernel page. */
	if ((kpg = getkpg(0)) == NULL)
		goto error0;
	
	/* Get free block in swap device. */
	blk = bitmap_first_free(swap.bitmap, (SWP_SIZE/PAGE_SIZE) >> 3);
	if (blk == BITMAP_FULL)
		goto error1;
	
	/*
	 * Set block on swap device as used
	 * in advance, because we may sleep below.
	 */
	off = HDD_SIZE + blk*PAGE_SIZE;
	bitmap_set(swap.bitmap, blk);
	
	/* Write page to disk. */
	kmemcpy(kpg, (void *)addr, PAGE_SIZE);
	n = bdev_write(SWAP_DEV, (void *)addr, PAGE_SIZE, off);
	if (n != PAGE_SIZE)
		goto error2;
	swap.count[blk]++;
	
	/* Set page as non-present. */
	pg->present = 0;
	pg->frame = blk;
	tlb_flush();
	
	putkpg(kpg);
	return (0);

error2:
	bitmap_clear(swap.bitmap, blk);
error1:
	putkpg(kpg);
error0:
	return (-1);
}

/**
 * @brief Swaps a page in to disk.
 * 
 * @param frame Frame number where the page should be placed.
 * @param addr  Address of the page to be swapped in.
 * 
 * @returns Zero upon success, and non-zero otherwise.
 */
PUBLIC int swap_in(unsigned frame, addr_t addr)
{
	unsigned blk;   /* Block number in swap device.  */
	struct pte *pg; /* Page table entry.             */
	off_t off;      /* Offset in swap device.        */
	ssize_t n;      /* # bytes read.                 */
	void *kpg;      /* Kernel page used for copying. */
	
	addr &= PAGE_MASK;
	pg = getpte(curr_proc, addr);
	
	/* Get kernel page. */
	if ((kpg = getkpg(0)) == NULL)
		goto error0;
	
	/* Get block # in swap device. */
	blk = pg->frame;
	off = HDD_SIZE + blk*PAGE_SIZE;
	
	/* Read page from disk. */
	n = bdev_read(SWAP_DEV, kpg, PAGE_SIZE, off);
	if (n != PAGE_SIZE)
		goto error1;
	swap_clear(pg);
	
	/* Set page as present. */
	pg->present = 1;
	pg->frame = (UBASE_PHYS >> PAGE_SHIFT) + frame;
	tlb_flush();
	
	/* Copy page. */
	kmemcpy((void *)addr, kpg, PAGE_SIZE);
	pg->accessed = 0;
	pg->dirty = 0;
		
	putkpg(kpg);
	return (0);

error1:
	putkpg(kpg);
error0:
	return (-1);
}

/**
 * @brief Increments the reference count of a swap page.
 */
PUBLIC void swap_inc(unsigned frame)
{
	 swap.count[frame]++;
}

#endif

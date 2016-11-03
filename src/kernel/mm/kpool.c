/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2016 Davidson Francis <davidsondfgl@gmail.com> 
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
#include <nanvix/hal.h>
#include <nanvix/mm.h>
#include <nanvix/klib.h>

/**
 * @brief Number of kernel pages.
 */
#define NR_KPAGES (KPOOL_SIZE/PAGE_SIZE)
 
/**
 * @brief Reference count for kernel pages.
 */
PRIVATE int kpages[NR_KPAGES] = { 0,  };

/**
 * @brief Translates a kernel page ID into a virtual address.
 *
 * @param id ID of target kernel page.
 *
 * @returns The virtual address of the target kernel page.
 */
PRIVATE inline addr_t kpg_id_to_addr(unsigned id)
{
	return (KPOOL_VIRT + (id << PAGE_SHIFT));
}

/**
 * @brief Translates a virtual address into a kernel page ID.
 *
 * @para vaddr Target virtual address.
 *
 * @returns The kernel page ID of the target virtual address.
 */
PRIVATE inline unsigned kpg_addr_to_id(addr_t addr)
{
	return ((addr - KPOOL_VIRT) >> PAGE_SHIFT);
}

/**
 * @brief Allocates a kernel page.
 * 
 * @param clean Should the page be cleaned?
 * 
 * @returns Upon success, a pointer to a kernel page is returned. Upon
 * failure, a NULL pointer is returned instead.
 */
PUBLIC void *getkpg(int clean)
{
	unsigned i; /* Loop index.  */
	void *kpg;  /* Kernel page. */
	
	/* Search for a free kernel page. */
	for (i = 0; i < NR_KPAGES; i++)
	{
		/* Found it. */
		if (kpages[i] == 0)
			goto found;
	}

	kprintf("mm: kernel page pool overflow");
	
	return (NULL);

found:

	/* Set page as used. */
	kpg = (void *) kpg_id_to_addr(i);
	kpages[i]++;
	
	/* Clean page. */
	if (clean)
		kmemset(kpg, 0, PAGE_SIZE);
	
	return (kpg);
}

/**
 * @brief Releases kernel page.
 * 
 * @param kpg Kernel page to be released.
 */
PUBLIC void putkpg(void *kpg)
{
	unsigned i;
	
	i = kpg_addr_to_id((addr_t) kpg);
	
	/* Double free. */
	if (kpages[i]-- == 0)
		kpanic("mm: double free on kernel page");
}

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

#ifndef I386_PAGING_H_
#define I386_PAGING_H_

	/* Shifts and masks. */
	#define PAGE_SHIFT  12                  /* Page shift.                 */
	#define PGTAB_SHIFT 22                  /* Page table shift.           */
	#define PAGE_MASK   (~(PAGE_SIZE - 1))  /* Page mask.                  */
	#define PGTAB_MASK  (~(PGTAB_SIZE - 1)) /* Page table mask.            */

	/* Sizes. */
	#define PAGE_SIZE  (1 << PAGE_SHIFT) /* Page size.                 */
	#define PGTAB_SIZE (1 << PGTAB_SHIFT) /* Page table size.           */
	#define PTE_SIZE   4                 /* Page table entry size.     */
	#define PDE_SIZE   4                 /* Page directory entry size. */

#ifndef _ASM_FILE_

	/*
	 * Page directory entry.
	 */
	struct pde
	{
		unsigned present  :  1; /* Present in memory? */
		unsigned writable :  1; /* Writable page?     */
		unsigned user     :  1; /* User page?         */
		unsigned          :  2; /* Reserved.          */
		unsigned accessed :  1; /* Accessed?          */
		unsigned dirty    :  1; /* Dirty?             */
		unsigned          :  2; /* Reserved.          */
		unsigned          :  3; /* Unused.            */
		unsigned frame    : 20; /* Frame number.      */
	};

	/*
	 * Page table entry.
	 */
	struct pte
	{
		unsigned present  :  1; /* Present in memory? */
		unsigned writable :  1; /* Writable page?     */
		unsigned user     :  1; /* User page?         */
		unsigned          :  2; /* Reserved.          */
		unsigned accessed :  1; /* Accessed?          */
		unsigned dirty    :  1; /* Dirty?             */
		unsigned          :  2; /* Reserved.          */
		unsigned cow      :  1; /* Copy on write?     */
		unsigned zero     :  1; /* Demand zero?       */
		unsigned fill     :  1; /* Demand fill?       */
		unsigned frame    : 20; /* Frame number.      */
	};

	/*
	 * DESCRIPTION;
	 *   The PG() macro returns the page number where a given virtual address.
	 */
	#define PG(a) (((unsigned)(a) & (PGTAB_MASK^PAGE_MASK)) >> PAGE_SHIFT)

	/*
	 * DESCRIPTION:
	 *   The PGTAB() macro returns the page table number of a given virtual
	 */
	#define PGTAB(a) ((unsigned)(a) >> PGTAB_SHIFT)

#endif /* _ASM_FILE_ */

#endif /* I386_PAGING_H_ */


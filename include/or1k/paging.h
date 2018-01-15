/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2017-2017 Davidson Francis <davidsondfgl@gmail.com>
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

#ifndef OR1K_PAGING_H_
#define OR1K_PAGING_H_
	
	/* Shifts and masks. */
	#define PAGE_SHIFT  13                  /* Page shift.                 */
	#define PGTAB_SHIFT 22                  /* Page table shift.           */
	#define PAGE_MASK   (~(PAGE_SIZE - 1))  /* Page mask.                  */
	#define PGTAB_MASK  (~(PGTAB_SIZE - 1)) /* Page table mask.            */

	/* Sizes. */
	#define PAGE_SIZE  (1 << PAGE_SHIFT)  /* Page size.                 */
	#define PGTAB_SIZE (1 << PGTAB_SHIFT) /* Page table size.           */
	#define PTE_SIZE   4                  /* Page table entry size.     */
	#define PT_SIZE    4096               /* Page table size.           */
	#define PT_SHIFT   10                 /* Page table shift.          */

	/* Page table entry constants. */
	#define PT_CC  0x1         /* Cache Coherency.       */
	#define PT_CI  0x2         /* Cache Inhibit.         */
	#define PT_WBC 0x4         /* Write-Back Cache.      */
	#define PT_WOM 0x8         /* Weakly-Ordered Memory. */
	#define PT_A   0x10        /* Accesed.               */
	#define PT_D   0x20        /* Dirty.                 */
	#define PT_PPI 0x1C0       /* Page Protection Index. */
	#define PT_L   0x200       /* Last.                  */
	#define PT_PPN 0xFFFFFC00  /* Physical Page Number.  */

	/* Page Protection Index, data. */
	#define PT_PPI_USR_RD   0x40  /* Supervisor Read/Write, User: Read.       */
	#define PT_PPI_USR_WR   0x80  /* Supervisor Read/Write, User: Write.      */
	#define PT_PPI_USR_RDWR 0xC0  /* Supervisor Read/Write, User: Read/Write. */

	/* Page Protection Index, instruction. */
	#define PT_PPI_USR_EX   0x80  /* Supervisor Execute, User: Execute. */	

#ifndef _ASM_FILE_

	/*
	 * Page table entry.
	 */
	#if 0
	struct pte
	{
		unsigned ccoherency :  1; /* Cache coherency?       */
		unsigned cinhibit   :  1; /* Cache inhibit?         */
		unsigned wbc        :  1; /* Write-back cache?      */
		unsigned wom        :  1; /* Weakly-orderer memory? */
		unsigned accessed   :  1; /* Accessed?              */
		unsigned dirty      :  1; /* Dirty?                 */
		unsigned ppi        :  3; /* Page protection index. */
		unsigned last       :  1; /* Last PTE.              */
		unsigned ppn        : 22; /* Physical Page Number.  */
	};
	#endif

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

	/**
	 * @brief Sets/clears the present bit of a page table directory entry.
	 *
	 * @param pde Target page table directory entry.
	 * @param set Set bit?
	 */
	static inline void pde_present_set(struct pde *pde, int set)
	{
		pde->present = (set) ? 1 : 0;
	}

	/**
	 * @brief Asserts if the present bit of a page table directory entry is set.
	 *
	 * @param pde Target page table directory entry.
	 *
	 * @returns Non zero if the present bit of the target page table
	 * directory entry is set, and false otherwise.
	 */
	static inline int pde_is_present(struct pde *pde)
	{
		return (pde->present);
	}

	/**
	 * @brief Sets/clears the write bit of a page table directory entry.
	 *
	 * @param pde Target page table directory entry.
	 * @param set Set bit?
	 */
	static inline void pde_write_set(struct pde *pde, int set)
	{
		pde->writable = (set) ? 1 : 0;
	}

	/**
	 * @brief Asserts if the write bit of a page table directory entry is
	 * set.
	 *
	 * @param pde Target page table directory entry.
	 *
	 * @returns Non zero if the write bit of the target page table
	 * directory entry is set, and false otherwise.
	 */
	static inline int pde_is_write(struct pde *pde)
	{
		return (pde->writable);
	}

	/**
	 * @brief Sets/clears the user bit of a page table directory entry.
	 *
	 * @param pde Target page table directory entry.
	 * @param set Set bit?
	 */
	static inline void pde_user_set(struct pde *pde, int set)
	{
		pde->user = (set) ? 1 : 0;
	}

	/**
	 * @brief Asserts if the user bit of a page table directory entry is set.
	 *
	 * @param pde Target page table entry.
	 *
	 * @returns Non zero if the user bit of the target page table
	 * directory entry is set, and false otherwise.
	 */
	static inline int pde_is_user(struct pde *pde)
	{
		return (pde->user);
	}

	/**
	 * @brief Sets/clears the present bit of a page table entry.
	 *
	 * @param pte Target page table entry.
	 * @param set Set bit?
	 */
	static inline void pte_present_set(struct pte *pte, int set)
	{
		pte->present = (set) ? 1 : 0;
	}

	/**
	 * @brief Asserts if the present bit of a page table entry is set.
	 *
	 * @param pte Target page table entry.
	 *
	 * @returns Non zero if the present bit of the target page table
	 * entry is set, and false otherwise.
	 */
	static inline int pte_is_present(struct pte *pte)
	{
		return (pte->present);
	}

	/**
	 * @brief Sets/clears the demand fill bit of a page table entry.
	 *
	 * @param pte Target page table entry.
	 * @param set Set bit?
	 */
	static inline void pte_fill_set(struct pte *pte, int set)
	{
		pte->fill = (set) ? 1 : 0;
	}

	/**
	 * @brief Asserts if the demand fill bit of a page table entry is
	 * set.
	 *
	 * @param pte Target page table entry.
	 *
	 * @returns Non zero if the demand fill bit of the target page
	 * table entry is set, and false otherwise.
	 */
	static inline int pte_is_fill(struct pte *pte)
	{
		return (pte->fill);
	}

	/**
	 * @brief Sets/clears the demand zero bit of a page table entry.
	 *
	 * @param pte Target page table entry.
	 * @param set Set bit?
	 */
	static inline void pte_zero_set(struct pte *pte, int set)
	{
		pte->zero = (set) ? 1 : 0;
	}

	/**
	 * @brief Asserts if the demand zero bit of a page table entry is
	 * set.
	 *
	 * @param pte Target page table entry.
	 *
	 * @returns Non zero if the demand zero bit of the target page
	 * table entry is set, and false otherwise.
	 */
	static inline int pte_is_zero(struct pte *pte)
	{
		return (pte->zero);
	}

	/**
	 * @brief Sets/clears the write bit of a page table entry.
	 *
	 * @param pte Target page table entry.
	 * @param set Set bit?
	 */
	static inline void pte_write_set(struct pte *pte, int set)
	{
		pte->writable = (set) ? 1 : 0;
	}

	/**
	 * @brief Asserts if the write bit of a page table entry is set.
	 *
	 * @param pte Target page table entry.
	 *
	 * @returns Non zero if the write bit of the target page table
	 * entry is set, and false otherwise.
	 */
	static inline int pte_is_write(struct pte *pte)
	{
		return (pte->writable);
	}

	/**
	 * @brief Sets/clears the user bit of a page table entry.
	 *
	 * @param pte Target page table entry.
	 * @param set Set bit?
	 */
	static inline void pte_user_set(struct pte *pte, int set)
	{
		pte->user = (set) ? 1 : 0;
	}

	/**
	 * @brief Asserts if the user bit of a page table entry is set.
	 *
	 * @param pte Target page table entry.
	 *
	 * @returns Non zero if the user bit of the target page table
	 * entry is set, and false otherwise.
	 */
	static inline int pte_is_user(struct pte *pte)
	{
		return (pte->user);
	}

	/**
	 * @brief Sets/clears the copy-on-write bit of a page table entry.
	 *
	 * @param pte Target page table entry.
	 * @param set Set bit?
	 */
	static inline void pte_cow_set(struct pte *pte, int set)
	{
		pte->cow = (set) ? 1 : 0;
	}

	/**
	 * @brief Asserts if the copy-on-write bit of a page table entry
	 * is set.
	 *
	 * @param pte Target page table entry.
	 *
	 * @returns Non zero if the copy-on-write bit of the target page
	 * table entry is set, and false otherwise.
	 */
	static inline int pte_is_cow(struct pte *pte)
	{
		return (pte->cow);
	}

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


/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * paging.h - Paging subsystem library.
 */

#ifndef NANVIX_PAGING_H_
#define NANVIX_PAGING_H_

	#include <i386/i386.h>
	#include <nanvix/const.h>
	#include <nanvix/pm.h>
	
	/* markpg() request types. */
	#define CLEAR 0 /* Clear page.                */
	#define FILL  1 /* Load page from executable. */
	
	/*
	 * Marks a page.
	 */
	EXTERN void markpg(struct pte *pg, int what);
	
	/*
	 * Allocates a kernel page.
	 */
	EXTERN void *getkpg(int clean);
	
	/*
	 * Releases a kernel page.
	 */
	EXTERN void putkpg(void *kpg);
	
	/*
	 * DESCRIPTION:
	 *   The mappgtab() function attempts to map the kernel page pointed to by 
	 *   kpg in the user address space, at address addr. The page is mapped as 
	 *   a read-only page table. If writable has a non zero value, the page is 
	 *   mapped as a readable and writable page table.
	 * 
	 * RETURN VALUE:
	 *   Upon successfull completion, the mappgtab() function returns zero. Upon
	 *   failure, a negative number is returned instead.
	 *
	 * ERRORS:
	 *   - Cannot map page table at addr.
	 */
	EXTERN int mappgtab(struct pte *pgdir, addr_t addr, void *kpg, int writable);
	
	/*
	 * DESCRIPTION:
	 *   The umappgtab() function unmaps the page table from the user address 
	 *   space. The page table to be unmapped is located at the page directory 
	 *   pointed to by pgdir and at address addr and shall match a page table 
	 *   that has been previously mapped by a prior call to mappgtab(). If it 
	 *   doesn't, the bahavior is undefined,
	 * 
	 * RETURN VALUE:
	 *   The umappgtab() function has no return value.
	 *
	 * ERRORS:
	 *   No errors are defined.
	 * 
	 * SEE ALSO:
	 *   mappgtab()
	 */
	EXTERN void umappgtab(struct pte *pgdir, addr_t addr);
	
	/*
	 * DESCRIPTION:
	 *   The allocupg() function allocates a user page and sets the page table 
	 *   entry pointed to by upg according. If writable has a non zero value the
	 *   allocated page is marked as writable.
	 *
	 * RETURN VALUE:
	 *   Upon sucessful completion, the allocupg() function returns zero. Upon
	 *   failure, a negative number is returned instead.
	 *
	 * ERRORS:
	 *   - There are no free user pages.
	 */
	EXTERN int allocupg(struct pte *upg, int writable);
	
	/*
	 * DESCRIPTION:
	 *   The freeupg() function frees the user page pointed to by upg. upg shall
	 *   match a user page previously allocated by a prior call to allocupg(). 
	 *   If if doesn't, the behavior is undefined.
	 *
	 * RETURN VALUE:
	 *   The freeupg() function has no return value.
	 *
	 * ERRORS:
	 *   No errors are defined.
	 * 
	 * SEE ALSO:
	 *   allocupg()
	 */
	EXTERN void freeupg(struct pte *upg);
	
	/*
	 * DESCRIPTION:
	 *   The cpypg() function copies the contents of the page pointed to by pg1
	 *   to the page pointed to by pg2.
	 * 
	 * RETURN VALUE:
	 *   The cpypg() function has no return value.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void cpypg(struct pte *pg1, struct pte *pg2);
	
	/*
	 * DESCRIPTION:
	 *   The linkupg() function links the user page pointed to by upg2 to the 
	 *   user page pointed to by upg1.
	 *
	 * RETURN VALUE:
	 *   The linkupg() function has no return value.
	 *
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void linkupg(struct pte *upg1, struct pte *upg2);
	
	/*
	 * DESCRIPTION:
	 *   The crtpgdir() function creates a page directory for the process
	 *   pointed to by proc.
	 *
	 * RETURN VALUE:
	 *   Upon successful completion, the crtpgdir() function returns zero. Upon
	 *   failure, a negative number is returned instead.
	 *
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN int crtpgdir(struct process *proc);
	
	/*
	 * DESCRIPTION:
	 *   The dstrypgdir() function destroys the page directory of the process
	 *   pointed to by proc. The process shall have a page directory that has
	 *   been previously created by a pior call to crtpgdir(). If it hasn't the
	 *   behavior is undefined.
	 * 
	 * RETURN VALUE:
	 *   The dstrypgdir() function has no return value.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void dstrypgdir(struct process *proc);
	
	/* 
	 * DESCRIPTION:
	 *   The vfault() function handles a validity page fault that has occourred 
	 *   at address addr.
	 * 
	 * RETURN VALUE:
	 *   The vfault() has no return value.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void vfault(addr_t addr);
	
	/*
	 * DESCRIPTION:
	 *   The pfault() function handles a protection page fault that has 
	 *   occourred at address addr.
	 * 
	 * RETURN VALUE:
	 *   The pfault() has no return value.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void pfault(addr_t addr);

#endif /* NANVIX_PAGING_H_ */

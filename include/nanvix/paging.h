/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <nanvix/paging.h> - Paging subsystem library.
 */

#ifndef NANVIX_PAGING_H_
#define NANVIX_PAGING_H_

	#include <i386/i386.h>
	#include <nanvix/const.h>
	#include <nanvix/pm.h>
	
	/*
	 * Allocates a kernel page.
	 */
	EXTERN void *getkpg(int clean);
	
	/*
	 * Releases a kernel page.
	 */
	EXTERN void putkpg(void *kpg);
	
	/*
	 * Maps a user page table.
	 */
	EXTERN int mappgtab(struct pde *pgdir, addr_t addr, void *kpg);
	
	/*
	 * Unmaps a user page table.
	 */
	EXTERN void umappgtab(struct pde *pgdir, addr_t addr);
	
	/*
	 * Allocates a user page.
	 */
	EXTERN int allocupg(struct pte *upg, int writable);
	
	/*
	 * Releases a user page.
	 */
	EXTERN void freeupg(struct pte *upg);
	
	/*
	 * Marks a page "demand zero".
	 */
	EXTERN void zeropg(struct pte *pg);
	
	/*
	 * Marks a page "demand fill"
	 */
	PUBLIC void fillpg(struct pte *pg);
	
	/*
	 * Links a user page to another.
	 */
	EXTERN void linkupg(struct pte *upg1, struct pte *upg2);
	
	/*
	 * Creates the page directory of a process.
	 */
	EXTERN int crtpgdir(struct process *proc);
	
	/*
	 * Destroys the page directory of a process.
	 */
	EXTERN void dstrypgdir(struct process *proc);
	
	/* 
	 * Handles a validity page fault.
	 */
	EXTERN int vfault(addr_t addr);
	
	/*
	 * Handles a protection page fault.
	 */
	EXTERN int pfault(addr_t addr);

#endif /* NANVIX_PAGING_H_ */

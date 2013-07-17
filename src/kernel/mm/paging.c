/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * paging.c - Paging subsystem.
 */

#include <asm/util.h>
#include <i386/i386.h>
#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/int.h>
#include <nanvix/mm.h>
#include <nanvix/paging.h>
#include <nanvix/region.h>
#include <signal.h>

/* Number of kernel pages. */
#define NR_KPAGES (KPOOL_SIZE/PAGE_SIZE)

/* Number of user pages. */
#define NR_UPAGES (UMEM_SIZE/PAGE_SIZE)

/* Kernel pages. */
PRIVATE int kpages[NR_KPAGES] = { 0,  };

/* User pages. */
PRIVATE int upages[NR_UPAGES] = { 0,  };

/*
 * Gets a kernel page from the kernel page pool.
 */
PUBLIC void *getkpg()
{
	int i;
	
	/* Search for a free kernel page. */
	for (i = 0; i < NR_KPAGES; i++)
	{
		/* Found it. */
		if (kpages[i] == 0)
			goto found;
	}

	kprintf("there are no free kernel pages");

	return (NULL);

found:

	/* Increment kernel page reference count. */
	kpages[i]++;
	
	return ((void *)(KPOOL_VIRT + i*PAGE_SIZE));
}

/*
 * Puts back a kernel page in the kernel page pool.
 */
PUBLIC void putkpg(void *kpg)
{
	int i;
	
	/* Clean page. */
	kmemset(kpg, 0, PAGE_SIZE);
	
	i = ((addr_t)kpg - KPOOL_VIRT)/PAGE_SIZE;
	
	/* Decrement kernel page reference count. */
	kpages[i]--;
	
	/* Double free. */
	if (kpages[i] < 0)
		kpanic("putting back kernel page twice");
}

/*
 * Maps a kernel page in the user address space as a page table.
 */
PUBLIC int mappgtab(struct pte *pgdir, addr_t addr, void *kpg, int writable)
{
	int i;
	
	i = addr >> PGTAB_SHIFT;
	
	/* Kernel page cannot be mapped at given address. */
	if ((addr >= KBASE_VIRT) || (pgdir[i].present))
	{
		kprintf("cannot map page table at %x", addr);
		return (-1);
	}
	
	/* Map kernel page. */
	pgdir[i].present = 1;
	pgdir[i].writable = (writable) ? 1 : 0;
	pgdir[i].user = 1;
	pgdir[i].frame = ((addr_t)kpg - KBASE_VIRT)/PAGE_SIZE;
	
	return (0);
}

/*
 * Unmaps a page table from the user address space.
 */
PUBLIC void umappgtab(struct pte *pgdir, addr_t addr)
{
	int i;
	
	i = addr >> PGTAB_SHIFT;
	
	/* Cannot unmap page at given address. */
	if ((addr >= KBASE_VIRT) || !(pgdir[i].present))
		kpanic("cannot unmap page table from kernel address space");
	
	/* Unmap kernel page. */
	kmemset(&pgdir[i], 0, sizeof(pgdir[i]));
}

/*
 * Allocates a user page.
 */
PUBLIC int allocupg(struct pte *upg, int writable)
{
	int i;
	
	/* Search for a free user page. */
	for (i = 0; i < NR_UPAGES; i++)
	{
		/* Found it. */
		if (upages[i] == 0)
			goto found;
	}
	
	kprintf("there are no free user pages");
	
	return (-1);

found:
	
	/* Increment user page reference count. */
	upages[i]++;
	
	upg->present = 1;
	upg->writable = (writable) ? 1 : 0;
	upg->user = 1;
	upg->frame = UBASE_PHYS/PAGE_SIZE + i;
	
	return (0);
}

/*
 * Frees a user page.
 */
PUBLIC void freeupg(struct pte *upg)
{
	int i;
	
	/* Nothing to be done. */
	if (!upg->present)
		return;
		
	i = upg->frame;
		
	/* Decrement user page reference count. */
	upages[i]--;
		
	/* Free user page. */
	kmemset(upg, 0, sizeof(upg));
		
	/* Double free. */
	if (upages[i] < 0)
		kpanic("freeing user page twice");
}

/*
 * Copies a page.
 */
PUBLIC void cpypg(struct pte *pg1, struct pte *pg2)
{
	pg1->present = pg2->present;
	pg1->writable = pg2->writable;
	pg1->user = pg2->user;
	pg1->cow = pg2->cow;
	physcpy(pg1->frame << PAGE_SHIFT, pg2->frame << PAGE_SHIFT, PAGE_SIZE);
}

/*
 * Links two user pages.
 */
PUBLIC void linkupg(struct pte *upg1, struct pte *upg2)
{	
	/* Page is not present. */
	if (!upg1->present)
		return;
	
	/* Set copy on write. */
	if (upg1->writable)
	{
		upg1->writable = 0;
		upg1->cow = 1;
	}
	
	/* Link pages. */
	upg2->present = upg1->present;
	upg2->writable = upg1->writable;
	upg2->user = upg1->user;
	upg2->cow = upg1->cow;
	upg2->frame = upg1->frame;

	/* Increment page reference count. */
	upages[upg1->frame]++;
}

/*
 * Creates a page directory for a process.
 */
PUBLIC int crtpgdir(struct process *proc)
{
	void *kstack;
	struct pte *pgdir;
	struct intstack *s1, *s2;
	
	pgdir = getkpg();
	
	/* Failed to get kernel page. */
	if (pgdir == NULL)
		goto err0;

	kstack = getkpg();
	
	/* Failed to get kernel page. */
	if (kstack == NULL)
		goto err1;

	/* Build page directory. */
	pgdir[0] = curr_proc->pgdir[0];
	pgdir[PGTAB(KBASE_VIRT)] = curr_proc->pgdir[PGTAB(KBASE_VIRT)];
	pgdir[PGTAB(KPOOL_VIRT)] = curr_proc->pgdir[PGTAB(KPOOL_VIRT)];
	
	/* Clone kernel stack. */
	kmemcpy(kstack, curr_proc->kstack, KSTACK_SIZE);
	
	proc->cr3 = (addr_t)pgdir - KBASE_VIRT;
	proc->pgdir = pgdir;
	proc->kstack = kstack;
	
	/* Adjust stack pointers. */
	proc->kesp = (curr_proc->kesp -(dword_t)curr_proc->kstack)+(dword_t)kstack;
	if (KERNEL_RUNNING(curr_proc))
	{
		s1 = (struct intstack *) curr_proc->kesp;
		s2 = (struct intstack *) proc->kesp;
		
		s2->ebp = (s1->ebp - (dword_t)curr_proc->kstack) + (dword_t)kstack;
	}
	return (0);

err1:
	putkpg(pgdir);
err0:
	return (-1);
}

/*
 * Destroys the page directory of a process.
 */
PUBLIC void dstrypgdir(struct process *proc)
{
	putkpg(proc->kstack);
	putkpg(proc->pgdir);
}

/*
 * Handles a validity page fault.
 */
PUBLIC void vfault(addr_t addr)
{
	int err;
	struct pte *pg;
	struct pregion *preg;
	
	preg = findreg(curr_proc, addr);
	
	/* Address outside virtual address space. */
	if (preg == NULL)
		goto err0;
	
	pg = &preg->reg->pgtab[PG(addr)];
	
	lockreg(preg->reg);

	err = allocupg(pg, preg->reg->mode & 0222);
	
	unlockreg(preg->reg);
		
	/* Failed to allocate user page. */
	if (err)
		goto err0;

	return;

err0:
	if (KERNEL_RUNNING(curr_proc))
	{
		if (curr_proc->flags & PROC_JMPSET)
			klongjmp(&curr_proc->kenv, -1);
			
		kpanic("kernel validity page fault");
	}
	else
		sndsig(curr_proc, SIGSEGV);
}

/*
 * Handles a protection page fault.
 */
PUBLIC void pfault(addr_t addr)
{
	int err;
	struct pte *pg;
	struct pte new_pg;
	struct pregion *preg;
	
	preg = findreg(curr_proc, addr);
	
	/* Address outside virtual address space. */
	if (preg == NULL)
	{
		sndsig(curr_proc, SIGSEGV);	
		return;
	}
	
	lockreg(preg->reg);
	
	pg = &preg->reg->pgtab[PG(addr)];

	/* Copy on write not enabled. */
	if (!pg->cow)
		goto err0;
		
	/* Duplicate page. */
	if (upages[pg->frame] > 1)
	{
		err = allocupg(&new_pg, pg->writable);
		
		/* Failed to allocate new user page. */
		if (err)
			goto err0;
			
		cpypg(pg, &new_pg);
		
		/* Unlik page. */
		upages[pg->frame]--;
	}
		
	/* Steal page. */
	else
	{
		pg->cow = 0;
		pg->writable = 1;
	}
		
	unlockreg(preg->reg);

	return;

err0:
	unlockreg(preg->reg);
	if (KERNEL_RUNNING(curr_proc))
	{
		if (curr_proc->flags & PROC_JMPSET)
			klongjmp(&curr_proc->kenv, -1);
			
		kpanic("kernel protection page fault");
	}
	else
		sndsig(curr_proc, SIGSEGV);
}

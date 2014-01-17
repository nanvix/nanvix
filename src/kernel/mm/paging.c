/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * paging.c - Paging subsystem.
 */

#include <asm/util.h>
#include <i386/i386.h>
#include <nanvix/const.h>
#include <nanvix/fs.h>
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
PUBLIC void *getkpg(void)
{
	int i;     /* Loop index.  */
	void *kpg; /* Kernel page. */
	
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

	/* Set page as used. */
	kpg = (void *)(KPOOL_VIRT + (i << PAGE_SHIFT));
	kpages[i]++;
	
	kmemset(kpg, 0, PAGE_SIZE);
	
	return (kpg);
}

/*
 * Puts back a kernel page in the kernel page pool.
 */
PUBLIC void putkpg(void *kpg)
{
	int i;
	
	i = ((addr_t)kpg - KPOOL_VIRT) >> PAGE_SHIFT;
	
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
	pgdir[i].frame = ((addr_t)kpg - KBASE_VIRT) >> PAGE_SHIFT;
	
	tlb_flush();
	
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
		kpanic("cannot unmap page table");

	/* Unmap kernel page. */
	kmemset(&pgdir[i], 0, sizeof(struct pte));
	
	tlb_flush();
}

/*
 * Marks a page.
 */
PUBLIC void markpg(struct pte *pg, int what)
{
	/* Load page from executable file. */
	if (what == FILL)
	{
		pg->fill = 1;
		pg->clear = 0;
	}
	
	/* Clear page. */
	else
	{
		pg->fill = 0;
		pg->clear = 1;
	}
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
		if (!upages[i])
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
	upg->frame = (UBASE_PHYS >> PAGE_SHIFT) + i;
	
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
		
	i = upg->frame - (UBASE_PHYS >> PAGE_SHIFT);
	
	/* Decrement user page reference count. */
	upages[i]--;
		
	/* Free user page. */
	kmemset(upg, 0, sizeof(struct pte));
		
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
	pg1->clear = pg2->clear;
	pg1->fill = pg2->fill;

	physcpy(pg1->frame << PAGE_SHIFT, pg2->frame << PAGE_SHIFT, PAGE_SIZE);
}

/*
 * Links two user pages.
 */
PUBLIC void linkupg(struct pte *upg1, struct pte *upg2)
{	
	/* Page is present. */
	if (upg1->present)
	{		
		/* Set copy on write. */
		if (upg1->writable)
		{
			upg1->writable = 0;
			upg1->cow = 1;
		}
		
		upages[upg1->frame - (UBASE_PHYS >> PAGE_SHIFT)]++;
	}
	
	kmemcpy(upg2, upg1, sizeof(struct pte));
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
	pgdir[PGTAB(INITRD_VIRT)] = curr_proc->pgdir[PGTAB(INITRD_VIRT)];
	
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
 * Reads page from inode.
 */
PRIVATE int readpg(struct pte *pg, struct region *reg, addr_t addr)
{
	void *kpg;           /* Auxiliary kernel page.   */
	struct inode *inode; /* File inode.              */
	
	/* Get auxiliary page. */
	if ((kpg = getkpg()) == NULL)
		return (-1);
		
	/* Read page. */
	inode = reg->file.inode;
	file_read(inode, kpg, PAGE_SIZE, reg->file.off + (PG(addr) << PAGE_SHIFT));
	
	/* Assign new user page. */
	if (allocupg(pg, reg->mode & MAY_WRITE))
	{
		putkpg(kpg);
		return (-1);
	}
	
	kmemcpy((void*)(addr & PAGE_MASK), kpg, PAGE_SIZE);
	pg->fill = 0;
	
	putkpg(kpg);
	return (0);
}

/*
 * Handles a validity page fault.
 */
PUBLIC void vfault(addr_t addr)
{
	struct pte *pg;       /* Working page.           */
	struct pregion *preg; /* Working process region. */
	struct region *reg;   /* Working region.         */

	/* Get associated region. */
	if ((preg = findreg(curr_proc, addr)) == NULL)
	{
		kpanic("debug 0");
		goto error0;
	}
	
	lockreg(reg = preg->reg);
	
	/* Outside virtual address space. */
	if (!withinreg(reg, addr))
	{			
		/* Not a stack region. */
		if (preg != STACK(curr_proc))
		{
			kpanic("debug 1");
			goto error0;
		}
	
		kprintf("growing stack");
		
		/* Expand region. */
		if (growreg(curr_proc, preg, (~PGTAB_MASK - reg->size) - (addr & ~PGTAB_MASK)))
		{
			kpanic("debug 2");
			goto error0;
		}
	}

	pg = &reg->pgtab[PG(addr)];
		
	/* Clear page. */
	if (pg->clear)
	{
		/* Assign new page to region. */
		if (allocupg(pg, reg->mode & MAY_WRITE))
		{
			kpanic("debug 3");
			goto error0;
		}
		
		kmemset((void *)(addr & PAGE_MASK), 0, PAGE_SIZE);
		pg->clear = 0;
	}
		
	/* Load page from executable file. */
	else if (pg->fill)
	{	
		/* Read page. */
		if (readpg(pg, reg, addr))
		{
			kpanic("debug 4");
			goto error0;
		}
	}
		
	/* Swap page in. */
	else
		kpanic("swap page in");
	
	unlockreg(reg);
	return;

error0:
	unlockreg(reg);
	if (KERNEL_RUNNING(curr_proc))
		kpanic("validity page fault");
	sndsig(curr_proc, SIGSEGV);
}

/*
 * Handles a protection page fault.
 */
PUBLIC void pfault(addr_t addr)
{
	int i;                /* Frame index.            */
	struct pte *pg;       /* Faulting page.          */
	struct pte new_pg;    /* New page.               */
	struct region *reg;   /* Working memory region.  */
	struct pregion *preg; /* Working process region. */
	
	preg = findreg(curr_proc, addr);
	
	/* Outside virtual address space. */
	if (preg == NULL)
		goto error0;
	
	lockreg(reg = preg->reg);
	
	pg = &reg->pgtab[PG(addr)];

	/* Copy on write not enabled. */
	if (!pg->cow)
		goto error1;

	i = pg->frame - (UBASE_PHYS >> PAGE_SHIFT);

	/* Duplicate page. */
	if (upages[i] > 1)
	{			
		/* Allocate new user page. */
		if (allocupg(&new_pg, pg->writable))
			goto error1;
			
		cpypg(&new_pg, pg);
		
		new_pg.cow = 0;
		new_pg.writable = 1;
		
		/* Unlik page. */
		upages[i]--;
		kmemcpy(pg, &new_pg, sizeof(struct pte));
	}
		
	/* Steal page. */
	else
	{
		pg->cow = 0;
		pg->writable = 1;
	}
		
	
	
	unlockreg(reg);

	return;

error1:
	unlockreg(reg);
error0:
	if (KERNEL_RUNNING(curr_proc))
		kpanic("kernel protection page fault");
	sndsig(curr_proc, SIGSEGV);
}

/*
 * Copyright(C) 2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <i386/i386.h>
#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/clock.h>
#include <nanvix/fs.h>
#include <nanvix/hal.h>
#include <nanvix/int.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/paging.h>
#include <nanvix/region.h>
#include <errno.h>
#include <signal.h>

/* Kernel pages. */
#define NR_KPAGES (KPOOL_SIZE/PAGE_SIZE) /* Amount.          */
PRIVATE int kpages[NR_KPAGES] = { 0,  }; /* Reference count. */

/* Number of user pages. */
#define NR_UPAGES (UMEM_SIZE/PAGE_SIZE)

/**
 * @brief Memory frames.
 */
PRIVATE struct
{
	unsigned count; /**< Reference count.             */
	unsigned age;   /**< Age.                         */
	pid_t owner;    /**< Page owner.                  */
	addr_t vaddr;   /**< Virtual address of the page. */
} frames[NR_UPAGES] = {{0, 0, 0, 0},  };

/*
 * Allocates a kernel page.
 */
PUBLIC void *getkpg(int clean)
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

	kprintf("mm: kernel page pool overflow");
	
	curr_proc->errno = -EAGAIN;
	return (NULL);

found:

	/* Set page as used. */
	kpg = (void *)(KPOOL_VIRT + (i << PAGE_SHIFT));
	kpages[i]++;
	
	/* Clean page. */
	if (clean)
		kmemset(kpg, 0, PAGE_SIZE);
	
	return (kpg);
}

/*
 * Releases kernel page.
 */
PUBLIC void putkpg(void *kpg)
{
	int i;
	
	i = ((addr_t)kpg - KPOOL_VIRT) >> PAGE_SHIFT;
	
	/* Release page. */
	kpages[i]--;
	
	/* Double free. */
	if (kpages[i] < 0)
		kpanic("mm: releasing kernel page twice");
}

/*
 * Maps a kernel page as a user page table.
 */
PUBLIC int mappgtab(struct pde *pgdir, addr_t addr, void *kpg)
{
	int i;
	
	i = PGTAB(addr);
	
	/* Another page table is mapped here. */
	if (pgdir[i].present)
	{
		curr_proc->errno = -EBUSY;
		return (-1);
	}
	
	/* Map kernel page. */
	pgdir[i].present = 1;
	pgdir[i].writable = 1;
	pgdir[i].user = 1;
	pgdir[i].frame = ((addr_t)kpg - KBASE_VIRT) >> PAGE_SHIFT;
	
	tlb_flush();
	
	return (0);
}

/*
 * Unmaps a page table from the user address space.
 */
PUBLIC void umappgtab(struct pde *pgdir, addr_t addr)
{
	int i;
	
	i = addr >> PGTAB_SHIFT;
	
	/* Unmap non present page table. */
	if (!(pgdir[i].present))
		kpanic("unmap non present page table");

	/* Unmap kernel page. */
	kmemset(&pgdir[i], 0, sizeof(struct pte));
	
	tlb_flush();
}

/**
 * @brief Allocates a frame.
 */
PRIVATE int alloc_frame(void)
{
	int i;     /* Loop index. */
	int older; /* Older page. */
	
	older = -1;
	
	/* Search for a free frame. */
	for (i = 0; i < NR_UPAGES; i++)
	{
		/* Found it. */
		if (frames[i].count == 0)
			goto found;
		
		/* Local page replacement policy. */
		if (frames[i].owner == curr_proc->pid)
		{
			if ((older < 0) || (frames[i].age < frames[older].age))
				older = i;
		}
	}
	
	/*
	 * There are no free user pages left for
	 * this process. If this happens too often,
	 * we'll have to implement a load balancing
	 * strategy.
	 */
	if (older < 0)
	{
		kprintf("mm: there are no free user pages left");
		return (-1);
	}
	
	return (-1);

found:

	frames[i].count++;
	
	return (i);
}

/**
 * @brief Allocates a user page.
 * 
 * @param upg      Page table entry that shall be filled.
 * @param writable Is the page writable?
 * 
 * @returns Zero upon success, and non-zero otherwise.
 */
PRIVATE int allocupg(addr_t addr, int writable)
{
	int i;             /* Page frame index.         */
	struct pte *pg;    /* Working page table entry. */
	struct pde *pgtab; /* Working page table entry. */
	
	/* Failed to allocate page frame. */
	if ((i = alloc_frame()) < 0)
	{
		curr_proc->errno = -EAGAIN;
		return (-1);
	}
	
	/* Find page table entry. */
	pgtab = &curr_proc->pgdir[PGTAB(addr)];
	pg = &((struct pte *)((pgtab->frame << PAGE_SHIFT) + KBASE_VIRT))[PG(addr)];
	
	/* Initialize page frame. */
	frames[i].age = ticks;
	frames[i].owner = curr_proc->pid;
	frames[i].vaddr = addr;
	
	/* Allocate page. */
	kmemset(pg, 0, sizeof(struct pte));
	pg->present = 1;
	pg->writable = (writable) ? 1 : 0;
	pg->user = 1;
	pg->frame = (UBASE_PHYS >> PAGE_SHIFT) + i;
	tlb_flush();
	
	return (0);
}

/*
 * Frees a user page.
 */
PUBLIC void freeupg(struct pte *upg)
{
	int i;
	
	/* In-disk page. */
	if (!upg->present)
	{
		kmemset(upg, 0, sizeof(struct pte));
		return;
	}
		
	i = upg->frame - (UBASE_PHYS >> PAGE_SHIFT);
		
	/* Double free. */
	if (frames[i].count == 0)
		kpanic("freeing user page twice");
	
	/* Free user page. */
	if (--frames[i].count)
		frames[i].owner = 0;
	kmemset(upg, 0, sizeof(struct pte));
	tlb_flush();
}

/*
 * Marks a page "demand zero".
 */
PUBLIC void zeropg(struct pte *pg)
{
	/* Bad page table entry. */
	if (pg->present)
		kpanic("demand zero on a present page");
	
	pg->fill = 0;
	pg->zero = 1;
}

/*
 * Marks a page "demand fill"
 */
PUBLIC void fillpg(struct pte *pg)
{
	/* Bad page table entry. */
	if (pg->present)
		kpanic("demand fill on a present page");
	
	pg->fill = 1;
	pg->zero = 0;	
}

/*
 * Links two user pages.
 */
PUBLIC void linkupg(struct pte *upg1, struct pte *upg2)
{	
	int i;
	
	/* Page is present. */
	if (upg1->present)
	{		
		/* Set copy on write. */
		if (upg1->writable)
		{
			upg1->writable = 0;
			upg1->cow = 1;
		}
		
		i = upg1->frame - (UBASE_PHYS >> PAGE_SHIFT);
		frames[i].count++;
	}
	
	kmemcpy(upg2, upg1, sizeof(struct pte));
}

/*
 * Creates a page directory for a process.
 */
PUBLIC int crtpgdir(struct process *proc)
{
	void *kstack;             /* Kernel stack.     */
	struct pde *pgdir;        /* Page directory.   */
	struct intstack *s1, *s2; /* Interrupt stacks. */
	
	/* Get kernel page for page directory. */
	pgdir = getkpg(1);
	if (pgdir == NULL)
		goto err0;

	/* Get kernel page for kernel stack. */
	kstack = getkpg(0);
	if (kstack == NULL)
		goto err1;

	/* Build page directory. */
	pgdir[0] = curr_proc->pgdir[0];
	pgdir[PGTAB(KBASE_VIRT)] = curr_proc->pgdir[PGTAB(KBASE_VIRT)];
	pgdir[PGTAB(KPOOL_VIRT)] = curr_proc->pgdir[PGTAB(KPOOL_VIRT)];
	pgdir[PGTAB(INITRD_VIRT)] = curr_proc->pgdir[PGTAB(INITRD_VIRT)];
	
	/* Clone kernel stack. */
	kmemcpy(kstack, curr_proc->kstack, KSTACK_SIZE);
	
	/* Adjust stack pointers. */
	proc->kesp = (curr_proc->kesp -(dword_t)curr_proc->kstack)+(dword_t)kstack;
	if (KERNEL_RUNNING(curr_proc))
	{
		s1 = (struct intstack *) curr_proc->kesp;
		s2 = (struct intstack *) proc->kesp;	
		s2->ebp = (s1->ebp - (dword_t)curr_proc->kstack) + (dword_t)kstack;
	}
	
	/* Assign page directory. */
	proc->cr3 = (addr_t)pgdir - KBASE_VIRT;
	proc->pgdir = pgdir;
	proc->kstack = kstack;
	
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
PRIVATE int readpg(struct region *reg, addr_t addr)
{
	char *p;             /* Read pointer. */
	off_t off;           /* Block offset. */
	ssize_t count;       /* Bytes read.   */
	struct inode *inode; /* File inode.   */
	struct pte *pg;    /* Working page table entry. */
	struct pde *pgtab; /* Working page table entry. */
	
	/* Assign new user page. */
	if (allocupg(addr, reg->mode & MAY_WRITE))
		return (-1);
	
	/* Find page table entry. */
	pgtab = &curr_proc->pgdir[PGTAB(addr)];
	pg = &((struct pte *)((pgtab->frame << PAGE_SHIFT) + KBASE_VIRT))[PG(addr)];
	
	/* Read page. */
	off = reg->file.off + (PG(addr) << PAGE_SHIFT);
	inode = reg->file.inode;
	p = (char *)(addr & PAGE_MASK);
	count = file_read(inode, p, PAGE_SIZE, off);
	
	/* Failed to read page. */
	if (count < 0)
	{
		freeupg(pg);
		return (-1);
	}
	
	/* Fill remainder bytes with zero. */
	else if (count < PAGE_SIZE)
		kmemset(p + count, 0, PAGE_SIZE - count);
	
	return (0);
}

/*
 * Handles a validity page fault.
 */
PUBLIC int vfault(addr_t addr)
{
	struct pte *pg;       /* Working page.           */
	struct region *reg;   /* Working region.         */
	struct pregion *preg; /* Working process region. */
	
	/* Get associated region. */
	preg = findreg(curr_proc, addr);
	if (preg == NULL)
		goto error0;
	
	lockreg(reg = preg->reg);
	
	/* Outside virtual address space. */
	if (!withinreg(preg, addr))
	{			
		/* Not a stack region. */
		if (preg != STACK(curr_proc))
			goto error1;
	
		kprintf("growing stack");
		
		/* Expand region. */
		if (growreg(curr_proc, preg, (~PGTAB_MASK - reg->size) - (addr & ~PGTAB_MASK)))
			goto error1;
	}

	pg = (reg->flags & REGION_DOWNWARDS) ?
		&reg->pgtab[REGION_PGTABS-(PGTAB(preg->start)-PGTAB(addr))-1][PG(addr)]: 
		&reg->pgtab[PGTAB(addr) - PGTAB(preg->start)][PG(addr)];
		
	/* Clear page. */
	if (pg->zero)
	{
		/* Assign new page to region. */
		if (allocupg(addr, reg->mode & MAY_WRITE))
			goto error1;
		
		kmemset((void *)(addr & PAGE_MASK), 0, PAGE_SIZE);
	}
		
	/* Load page from executable file. */
	else if (pg->fill)
	{
		/* Read page. */
		if (readpg(reg, addr))
			goto error1;
	}
		
	/* Swap page in. */
	else
		kpanic("swap page in");
	
	unlockreg(reg);
	return (0);

error1:
	unlockreg(reg);
error0:
	return (-1);
}

/*
 * Copies a page.
 */
PRIVATE int cpypg(struct pte *pg1, struct pte *pg2)
{
	int i;
	
	/* Allocate new user page. */
	if ((i = alloc_frame()) < 0)
		return (-1);
	
	pg1->present = pg2->present;
	pg1->writable = pg2->writable;
	pg1->user = pg2->user;
	pg1->cow = pg2->cow;
	pg1->frame = (UBASE_PHYS >> PAGE_SHIFT) + i;

	physcpy(pg1->frame << PAGE_SHIFT, pg2->frame << PAGE_SHIFT, PAGE_SIZE);
	
	return (0);
}

/*
 * Handles a protection page fault.
 */
PUBLIC int pfault(addr_t addr)
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

	pg = (reg->flags & REGION_DOWNWARDS) ?
		&reg->pgtab[REGION_PGTABS-(PGTAB(preg->start)-PGTAB(addr))-1][PG(addr)]: 
		&reg->pgtab[PGTAB(addr) - PGTAB(preg->start)][PG(addr)];

	/* Copy on write not enabled. */
	if (!pg->cow)
		goto error1;

	i = pg->frame - (UBASE_PHYS >> PAGE_SHIFT);

	/* Duplicate page. */
	if (frames[i].count > 1)
	{
		if (cpypg(&new_pg, pg))
			goto error1;
		
		new_pg.cow = 0;
		new_pg.writable = 1;
		
		/* Unlik page. */
		frames[i].count--;
		kmemcpy(pg, &new_pg, sizeof(struct pte));
	}
		
	/* Steal page. */
	else
	{
		pg->cow = 0;
		pg->writable = 1;
	}
	
	unlockreg(reg);
	return(0);

error1:
	unlockreg(reg);
error0:
	return (-1);
}

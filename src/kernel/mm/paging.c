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
#include <nanvix/clock.h>
#include <nanvix/dev.h>
#include <nanvix/fs.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/region.h>
#include <signal.h>
#include "mm.h"

/**
 * @brief Gets a page directory entry of a process.
 * 
 * @param p Process.
 * @param a Address.
 * 
 * @returns The requested page directory entry.
 */
#define getpde(p, a) \
	(&(p)->pgdir[PGTAB(a)])

/**
 * @brief Gets a page table entry of a process.
 * 
 * @param p Process.
 * @param a Address.
 * 
 * @returns The requested page table entry.
 */
#define getpte(p, a) \
	(&((struct pte *)((getpde(p, a)->frame << PAGE_SHIFT) + KBASE_VIRT))[PG(a)])

/*============================================================================*
 *                             Kernel Page Pool                               *
 *============================================================================*/

/* Kernel pages. */
#define NR_KPAGES (KPOOL_SIZE/PAGE_SIZE) /* Number of kernel pages.  */
PRIVATE int kpages[NR_KPAGES] = { 0,  }; /* Reference count.         */

/**
 * @brief Allocates a kernel page.
 * 
 * @param clean Should the page be cleaned?
 * 
 * @returns Upon success, a pointer to a page is returned. Upon failure, a NULL
 *          pointer is returned instead.
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
	kpg = (void *)(KPOOL_VIRT + (i << PAGE_SHIFT));
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
	
	i = ((addr_t)kpg - KPOOL_VIRT) >> PAGE_SHIFT;
	
	/* Release page. */
	kpages[i]--;
	
	/* Double free. */
	if (kpages[i] < 0)
		kpanic("mm: releasing kernel page twice");
}

/*============================================================================*
 *                              Paging System                                 *
 *============================================================================*/

/* Number of page frames. */
#define NR_FRAMES (UMEM_SIZE/PAGE_SIZE)

/**
 * @brief Page frames.
 */
PRIVATE struct
{
	unsigned count; /**< Reference count.     */
	pid_t owner;    /**< Page owner.          */
	addr_t addr;    /**< Address of the page. */
} frames[NR_FRAMES] = {{0, 0, 0},  };

/**
 * @brief Allocates a page frame.
 * 
 * @returns Upon success, the number of the frame is returned. Upon failure, a
 *          negative number is returned instead.
 */
PRIVATE int allocf(void)
{
	/* Search for a free frame. */
	for (int i = 0; i < NR_FRAMES; i++)
	{
		/* Found it. */
		if (frames[i].count == 0)
		{
			frames[i].count = 1;
			return (i);
		}
	}
	
	return (-1);
}

/**
 * @brief Copies a page.
 * 
 * @brief pg1 Target page.
 * @brief pg2 Source page.
 * 
 * @returns Zero upon successful completion, and non-zero otherwise.
 * 
 * @note The source page is assumed to be in-core.
 */
PRIVATE int cpypg(struct pte *pg1, struct pte *pg2)
{
	int i;
	
	/* Allocate new user page. */
	if ((i = allocf()) < 0)
		return (-1);
	
	/* Handcraft page table entry. */
	pg1->present = pg2->present;
	pg1->writable = pg2->writable;
	pg1->user = pg2->user;
	pg1->cow = pg2->cow;
	pg1->frame = (UBASE_PHYS >> PAGE_SHIFT) + i;

	physcpy(pg1->frame << PAGE_SHIFT, pg2->frame << PAGE_SHIFT, PAGE_SIZE);
	
	return (0);
}

/**
 * @brief Allocates a user page.
 * 
 * @param addr     Address where the page resides.
 * @param writable Is the page writable?
 * 
 * @returns Zero upon successful completion, and non-zero otherwise.
 */
PRIVATE int allocupg(addr_t addr, int writable)
{
	int i;          /* Page frame index.         */
	addr_t paddr;   /* Page address.             */
	struct pte *pg; /* Working page table entry. */
	
	/* Failed to allocate page frame. */
	if ((i = allocf()) < 0)
		return (-1);
	
	paddr = addr & PAGE_MASK;

	/* Initialize page frame. */
	frames[i].owner = curr_proc->pid;
	frames[i].addr = paddr;
	
	/* Allocate page. */
	pg = getpte(curr_proc, addr);
	kmemset(pg, 0, sizeof(struct pte));
	pg->present = 1;
	pg->writable = (writable) ? 1 : 0;
	pg->user = 1;
	pg->frame = (UBASE_PHYS >> PAGE_SHIFT) + i;
	tlb_flush();
	
	kmemset((void *)paddr, 0, PAGE_SIZE);
	
	return (0);
}

/**
 * @brief Reads a page from a file.
 * 
 * @param reg  Region where the page resides.
 * @param addr Address where the page should be loaded. 
 * 
 * @returns Zero upon successful completion, and non-zero upon failure.
 */
PRIVATE int readpg(struct region *reg, addr_t addr)
{
	char *p;             /* Read pointer.             */
	off_t off;           /* Block offset.             */
	ssize_t count;       /* Bytes read.               */
	struct inode *inode; /* File inode.               */
	struct pte *pg;      /* Working page table entry. */
	
	addr &= PAGE_MASK;
	
	/* Assign a user page. */
	if (allocupg(addr, reg->mode & MAY_WRITE))
		return (-1);
	
	/* Find page table entry. */
	pg = getpte(curr_proc, addr);
	
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

/**
 * @brief Maps a page table into user address space.
 * 
 * @param proc  Process in which the page table should be mapped.
 * @param addr  Address where the page should be mapped.
 * @param pgtab Page table to map.
 */
PUBLIC void mappgtab(struct process *proc, addr_t addr, void *pgtab)
{
	struct pde *pde;
	
	pde = &proc->pgdir[PGTAB(addr)];
	
	/* Bad page table. */
	if (pde->present)
		kpanic("busy page table entry");
	
	/* Map kernel page. */
	pde->present = 1;
	pde->writable = 1;
	pde->user = 1;
	pde->frame = (ADDR(pgtab) - KBASE_VIRT) >> PAGE_SHIFT;
	
	/* Flush changes. */
	if (proc == curr_proc)
		tlb_flush();
}

/**
 * @brief Unmaps a page table from user address space.
 * 
 * @param proc Process in which the page table should be unmapped.
 * @param addr Address where the page should be unmapped.
 * 
 * @returns Zero upon success, and non zero otherwise.
 */
PUBLIC void umappgtab(struct process *proc, addr_t addr)
{
	struct pde *pde;
	
	pde = &proc->pgdir[PGTAB(addr)];
	
	/* Bad page table. */
	if (!(pde->present))
		kpanic("unmap non-present page table");

	/* Unmap kernel page. */
	kmemset(pde, 0, sizeof(struct pde));
	
	/* Flush changes. */
	if (proc == curr_proc)
		tlb_flush();
}

/**
 * @brief Frees a user page.
 * 
 * @param pg Page to be freed.
 */
PUBLIC void freeupg(struct pte *pg)
{
	unsigned i;

	/* Do nothing. */
	if (*((unsigned *) pg) == 0)
		return;
	
	/* Check for demand page. */
	if (!pg->present)
	{
		/* Demand page. */
		if (pg->fill || pg->zero)
		{
			kmemset(pg, 0, sizeof(struct pte));
			return;
		}
		kpanic("freeing invalid user page");
	}
		
	i = pg->frame - (UBASE_PHYS >> PAGE_SHIFT);
		
	/* Double free. */
	if (frames[i].count == 0)
		kpanic("freeing user page twice");
	
	/* Free user page. */
	if (--frames[i].count)
		frames[i].owner = 0;
	kmemset(pg, 0, sizeof(struct pte));
	tlb_flush();
}

/**
 * @brief Marks a page.
 * 
 * @param pg   Page to be marked.
 * @param mark Mark.
 */
PUBLIC void markpg(struct pte *pg, int mark)
{
	/* Bad page. */
	if (pg->present)
		kpanic("demand fill on a present page");
	
	/* Mark page. */
	switch (mark)
	{
		/* Demand fill. */
		case PAGE_FILL:
			pg->fill = 1;
			pg->zero = 0;
			break;
		
		/* Demand zero. */
		case PAGE_ZERO:
			pg->fill = 0;
			pg->zero = 1;
			break;
	}
}

/**
 * @brief Enables/disable copy-on-write on a page.
 *
 * @param pg     Target page.
 * @param enable Enable copy-on-write?
 */
PRIVATE void pgcow(struct pte *pg, int enable)
{
	/* Disable cow. */
	if (!enable)
	{
		pg->cow = 0;
		pg->writable = 1;
		return;
	}

	pg->cow = 1;
	pg->writable = 0;
}

/**
 * @brief Links two pages.
 * 
 * @param upg1 Source page.
 * @param upg2 Target page.
 */
PUBLIC void linkupg(struct pte *upg1, struct pte *upg2)
{	
	unsigned i;

	/* Nothing to do. */
	if (*((unsigned *) upg1) == 0)
		return;

	/* Invalid. */
	if (!upg1->present)
	{
		/* Demand page. */
		if (upg1->fill || upg1->zero)
		{
			kmemcpy(upg2, upg1, sizeof(struct pte));
			return;
		}

		kpanic("linking invalid user page");
	}

	/* Set copy on write. */
	if (upg1->writable)
		pgcow(upg1, 1);

	i = upg1->frame - (UBASE_PHYS >> PAGE_SHIFT);
	frames[i].count++;
	
	kmemcpy(upg2, upg1, sizeof(struct pte));
}

/**
 * @brief Creates a page directory for a process.
 * 
 * @param proc Target process.
 * 
 * @returns Upon successful completion, zero is returned. Upon failure, non-zero
 *          is returned instead.
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
	proc->cr3 = ADDR(pgdir) - KBASE_VIRT;
	proc->pgdir = pgdir;
	proc->kstack = kstack;
	
	return (0);

err1:
	putkpg(pgdir);
err0:
	return (-1);
}

/**
 * @brief Destroys the page directory of a process.
 * 
 * @param proc Target process.
 * 
 * @note The current running process may not be the target process.
 */
PUBLIC void dstrypgdir(struct process *proc)
{
	putkpg(proc->kstack);
	putkpg(proc->pgdir);
}

/**
 * @brief Handles a validity page fault.
 * 
 * @brief addr Faulting address.
 * 
 * @returns Upon successful completion, zero is returned. Upon
 * failure, non-zero is returned instead.
 */
PUBLIC int vfault(addr_t addr)
{
	struct pte *pg;       /* Working page.           */
	struct region *reg;   /* Working region.         */
	struct pregion *preg; /* Working process region. */

	/* Get process region. */
	if ((preg = findreg(curr_proc, addr)) != NULL)
		lockreg(reg = preg->reg);
	else
	{
		addr_t addr2;

		addr2 = addr + PAGE_SIZE;

		/* Check for stack growth. */
		if ((preg = findreg(curr_proc, addr2)) == NULL)
			goto error0;

		lockreg(reg = preg->reg);

		/* Not a stack region. */
		if (preg != STACK(curr_proc))
			goto error1;

		/* Expand region. */
		if (growreg(curr_proc, preg, PAGE_SIZE))
			goto error1;
	}
	
	pg = getpte(curr_proc, addr);
		
	/* Not demand zero. */
	if (!pg->zero)
	{
		/* Not demand fill. */
		if (!pg->fill)
			goto error1;

		/* Demand fill. */
		if (readpg(reg, addr))
			goto error1;
		
		goto ok;
	}
		
	/* Demand zero. */
	if (allocupg(addr, reg->mode & MAY_WRITE))
		goto error1;

ok:

	unlockreg(reg);
	return (0);

error1:
	unlockreg(reg);
error0:
	return (-1);
}

/**
 * @brief Handles a protection page fault.
 * 
 * @brief addr Faulting address.
 * 
 * @returns Upon successful completion, zero is returned. Upon failure, non-zero
 *          is returned instead.
 */
PUBLIC int pfault(addr_t addr)
{
	unsigned i;           /* Frame index.            */
	struct pte *pg;       /* Faulting page.          */
	struct pte new_pg;    /* New page.               */
	struct pregion *preg; /* Working process region. */

	/* Outside virtual address space. */
	if ((preg = findreg(curr_proc, addr)) == NULL)
		goto error0;
	
	lockreg(preg->reg);

	pg = getpte(curr_proc, addr);

	/* Copy on write not enabled. */
	if (!pg->cow)
		goto error1;
		
	i = pg->frame - (UBASE_PHYS >> PAGE_SHIFT);

	/* Steal page. */
	if (frames[i].count == 1)
	{
		pgcow(pg, 0);
		goto ok;
	}
		
	/* Copy page. */
	if (cpypg(&new_pg, pg))
		goto error1;
	
	pgcow(&new_pg, 0);
	
	/* Unlik page. */
	frames[i].count--;
	kmemcpy(pg, &new_pg, sizeof(struct pte));

ok:
	unlockreg(preg->reg);
	return(0);

error1:
	unlockreg(preg->reg);
error0:
	return (-1);
}

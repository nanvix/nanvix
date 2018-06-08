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
#include <nanvix/fs.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/region.h>
#include "mm.h"

/*============================================================================*
 *                             Page Frames Subsystem                          *
 *============================================================================*/

/**
 * @brief Number of page frames.
 */
#define NR_FRAMES (UMEM_SIZE/PAGE_SIZE)

/**
 * @brief Reference count for page frames.
 */
PRIVATE unsigned frames[NR_FRAMES] = {0, };

/**
 * @brief Converts a frame ID to a frame number.
 *
 * @param id ID of target page frame.
 *
 * @returns Frame number of target page frame.
 */
PRIVATE inline addr_t frame_id_to_addr(unsigned id)
{
	return ((UBASE_PHYS >> PAGE_SHIFT) + id);
}

/**
 * @brief Converts a frame number to a frame ID.
 *
 * @param addr Frame number of target page frame
 *
 * @returns ID of target page frame.
 */
PRIVATE inline int frame_addr_to_id(addr_t addr)
{
	return (addr - (UBASE_PHYS >> PAGE_SHIFT));
}

/**
 * @brief Allocates a page frame.
 * 
 * @returns The page frame number upon success, and zero upon failure.
 */
PRIVATE addr_t frame_alloc(void)
{
	/* Search for a free frame. */
	for (unsigned i = 0; i < NR_FRAMES; i++)
	{
		/* Found it. */
		if (frames[i] == 0)
		{
			frames[i] = 1;
			
			return (frame_id_to_addr(i));
		}
	}
	
	return (0);
}

/**
 * @brief Frees a page frame.
 *
 * @param addr Frame number of target page frame.
 */
PRIVATE inline void frame_free(addr_t addr)
{
	if (frames[frame_addr_to_id(addr)]-- == 0)
		kpanic("mm: double free on page frame");
}

/**
 * @brief Increments the reference count of a page frame.
 *
 * @param i ID of target page frame.
 */
PRIVATE inline void frame_share(addr_t addr)
{
	frames[frame_addr_to_id(addr)]++;
}

/**
 * @brief Asserts if a page frame is begin shared.
 *
 * @param addr Frame number of target page frame.
 *
 * @returns Non zero if the page frame is being shared, and zero
 * otherwise.
 */
PRIVATE inline int frame_is_shared(addr_t addr)
{
	return (frames[frame_addr_to_id(addr)] > 1);
}

/*============================================================================*
 *                              Paging System                                 *
 *============================================================================*/

/**
 * @brief Gets a page directory entry.
 * 
 * @param proc Target process.
 * @param addr Target address.
 * 
 * @returns The requested page directory entry.
 */
PRIVATE inline struct pde *getpde(struct process *proc, addr_t addr)
{
	return (&proc->pgdir[PGTAB(addr)]);
}

/**
 * @brief Gets a page table entry of a process.
 * 
 * @param proc Target process.
 * @param addr Target address.
 * 
 * @returns The requested page table entry.
 */
PRIVATE inline struct pte *getpte(struct process *proc, addr_t addr)
{
	addr_t base;

	base = (getpde(proc, addr)->frame << PAGE_SHIFT) + KBASE_VIRT;

	return (&((struct pte *) base)[PG(addr)]);
}

/**
 * @brief Initializes a page table directory entry.
 *
 * @param pde Target page table directory entry.
 */
PRIVATE inline void pde_init(struct pde *pde)
{
	pde_present_set(pde, 1);
	pde_user_set(pde, 1);
	pde_write_set(pde, 1);
}

/**
 * @brief Clears a page table directory entry.
 *
 * @param pde Target page table directory entry.
 */
PRIVATE inline void pde_clear(struct pde *pde)
{
	pde_present_set(pde, 0);
	pde_user_set(pde, 0);
	pde_write_set(pde, 0);
}

/**
 * @brief Asserts if a page table directory entry is cleared.
 *
 * @param pde Target page table directory entry.
 */
PRIVATE inline int pde_is_clear(struct pde *pde)
{
	return (pde_is_present(pde));
}

/**
 * @brief Initializes a page table entry.
 *
 * @param pte      Target page table entry.
 * @param writable Is target page table entry writable?
 */
PRIVATE inline void pte_init(struct pte *pte, int writable)
{
	pte_present_set(pte, 1);
	pte_cow_set(pte, 0);
	pte_zero_set(pte, 0);
	pte_fill_set(pte, 0);
	pte_user_set(pte, 1);
	pte_write_set(pte, writable);
}

/**
 * @brief Clears a page table entry.
 *
 * @param pte Target page table entry.
 */
PRIVATE inline void pte_clear(struct pte *pte)
{
	pte_present_set(pte, 0);
	pte_cow_set(pte, 0);
	pte_zero_set(pte, 0);
	pte_fill_set(pte, 0);
}

/**
 * @brief Asserts if a page table entry is cleared.
 *
 * @param pte Target page table entry.
 *
 * @returns Non zero if the page is cleared, and zero otherwise.
 */
PRIVATE inline int pte_is_clear(struct pte *pte)
{
	return (!(pte_is_present(pte) | pte_is_fill(pte) | pte_is_zero(pte)));
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
	if (pde_is_clear(pde))
		kpanic("mm: busy page table directory entry");
	
	/* Map kernel page. */
	pde_init(pde);
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
	if (!(pde_is_clear(pde)))
		kpanic("mm: invalid page table directory entry");

	/* Unmap kernel page. */
	pde_clear(pde);
	
	/* Flush changes. */
	if (proc == curr_proc)
		tlb_flush();
}

/**
 * @brief Creates a page directory for a process.
 * 
 * @param proc Target process.
 * 
 * @returns Upon successful completion, zero is returned. Upon
 * failure, non-zero is returned instead.
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
	pgdir[PGTAB(SERIAL_VIRT)] = curr_proc->pgdir[PGTAB(SERIAL_VIRT)];
	
	/* Clone kernel stack. */
	kmemcpy(kstack, curr_thread->kstack, KSTACK_SIZE);
	/* Adjust stack pointers. */
	proc->threads->kesp = (curr_thread->kesp -(dword_t)curr_thread->kstack)+(dword_t)kstack;
	s1 = (struct intstack *) proc->threads->kesp;
	s1->old_kesp = proc->threads->kesp;
	
	if (curr_proc == IDLE)
	{
		s1 = (struct intstack *) curr_thread->kesp;
		s2 = (struct intstack *) proc->threads->kesp;
#ifdef i386
		s2->ebp = (s1->ebp - (dword_t)curr_thread->kstack) + (dword_t)kstack;
#elif or1k
		s2->gpr[2] = (s1->gpr[2] - (dword_t)curr_thread->kstack) + (dword_t)kstack;
#endif
	}

	/* Assign page directory. */
	proc->cr3 = ADDR(pgdir) - KBASE_VIRT;
	proc->pgdir = pgdir;
	proc->threads->kstack = kstack;
	
	return (0);

err1:
	putkpg(pgdir);
err0:
	return (-1);
}

/**
 * @brief Clones a page.
 * 
 * @brief pg Target page.
 * 
 * @returns Zero upon successful completion, and non-zero otherwise.
 */
PRIVATE int clonepg(struct pte *pg)
{
	addr_t newframe; /* New page frame. */
	addr_t oldframe; /* Old page frame. */
	
	/* Grab a page frame. */
	if (!(newframe = frame_alloc()))
		return (-1);
	
	/* Unlink old frame. */
	oldframe = pg->frame;
	pg->frame = newframe;
	frame_free(oldframe);

	physcpy(newframe << PAGE_SHIFT, oldframe << PAGE_SHIFT, PAGE_SIZE);
	
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
PRIVATE int allocupg(addr_t vaddr, int writable)
{
	addr_t paddr;   /* Page address.             */
	struct pte *pg; /* Working page table entry. */
	
	/* Failed to allocate page frame. */
	if (!(paddr = frame_alloc()))
		return (-1);

	vaddr &= PAGE_MASK;
	
	/* Allocate page. */
	pg = getpte(curr_proc, vaddr);
	pte_init(pg, writable);
	pg->frame = paddr;
	tlb_flush();
	
	kmemset((void *)(vaddr), 0, PAGE_SIZE);
	
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
	p = (char *)(addr);
	count = file_read(inode, p, PAGE_SIZE, off);
	
	/* Failed to read page. */
	if (count < 0)
	{
		freeupg(pg);
		return (-1);
	}
	
	return (0);
}

/**
 * @brief Frees a user page.
 * 
 * @param pg Page to be freed.
 */
PUBLIC void freeupg(struct pte *pg)
{
	/* Do nothing. */
	if (pte_is_clear(pg))
		return;
	
	/* Check for demand page. */
	if (!pte_is_present(pg))
	{
		/* Demand page. */
		if (pte_is_fill(pg) || pte_is_zero(pg))
			goto done;

		kpanic("mm: freeing invalid user page");
	}
		
	frame_free(pg->frame);

done:
	pte_clear(pg);
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
	if (pte_is_present(pg))
		kpanic("mm: demand fill on a present page");
	
	/* Mark page. */
	switch (mark)
	{
		/* Demand fill. */
		case PAGE_FILL:
			pte_fill_set(pg, 1);
			pte_zero_set(pg, 0);
			break;
		
		/* Demand zero. */
		case PAGE_ZERO:
			pte_fill_set(pg, 0);
			pte_zero_set(pg, 1);
			break;
	}
}

/**
 * @brief Enables copy-on-write on a page.
 *
 * @param pg Target page.
 */
PRIVATE void cow_enable(struct pte *pg)
{
	pte_cow_set(pg, 1);
	pte_write_set(pg, 0);
}

/**
 * @brief Disables copy-on-write on a page.
 *
 * @param pg Target page.
 *
 * @returns Zero on success, and non zero otherwise.
 */
PRIVATE int cow_disable(struct pte *pg)
{
	/* Steal page. */
	if (frame_is_shared(pg->frame))
	{
		/* Clone page. */
		if (clonepg(pg))
			return (-1);
	}

	pte_cow_set(pg, 0);
	pte_write_set(pg, 1);
	tlb_flush();

	return (0);
}

/**
 * @brief Asserts if copy-on-write is enabled on a page.
 *
 * @param pg Target page.
 *
 * @returns Non zero if copy-on-write is enabled, and zero otherwise.
 */
PRIVATE int cow_is_enabled(struct pte *pg)
{
	return ((pte_is_cow(pg)) && (!pte_is_write(pg)));
}

/**
 * @brief Links two pages.
 * 
 * @param upg1 Source page.
 * @param upg2 Target page.
 */
PUBLIC void linkupg(struct pte *upg1, struct pte *upg2)
{	
	/* Nothing to do. */
	if (pte_is_clear(upg1))
		return;

	/* Invalid. */
	if (!pte_is_present(upg1))
	{
		/* Demand page. */
		if (pte_is_fill(upg1) || pte_is_zero(upg1))
		{
			kmemcpy(upg2, upg1, sizeof(struct pte));
			return;
		}

		kpanic("linking invalid user page");
	}

	/* Set copy on write. */
	if (pte_is_write(upg1))
		cow_enable(upg1);

	frame_share(upg1->frame);
	
	kmemcpy(upg2, upg1, sizeof(struct pte));
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
	putkpg(proc->threads->kstack);
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
	struct thread *thrd;  /* Working thread.         */

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
		thrd = curr_proc->threads;
		while (thrd != NULL)
		{
			if (preg == &thrd->pregs)
				goto stack_reg;
			thrd = thrd->next;
		}
		goto error1;
stack_reg:

		/* Expand region. */
		if (growreg(curr_proc, preg, PAGE_SIZE))
			goto error1;
	}
	
	pg = getpte(curr_proc, addr);
	
	/* Should be demand fill or demand zero. */
	if (!(pte_is_fill(pg) || pte_is_zero(pg)))
		goto error1;
	
	/* Demand fill. */
	else if (pte_is_fill(pg))
	{
		if (readpg(reg, addr))
			goto error1;
	}

	/* Demand zero. */
	else
	{
		if (allocupg(addr, reg->mode & MAY_WRITE))
			goto error1;
	}

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
 * @returns Upon successful completion, zero is returned. Upon
 * failure, non-zero is returned instead.
 */
PUBLIC int pfault(addr_t addr)
{
	struct pte *pg;       /* Faulting page.          */
	struct pregion *preg; /* Working process region. */

	/* Outside virtual address space. */
	if ((preg = findreg(curr_proc, addr)) == NULL)
		goto error0;
	
	lockreg(preg->reg);

	pg = getpte(curr_proc, addr);

	/* Copy on write not enabled. */
	if (!cow_is_enabled(pg))
		goto error1;
		
	/* Copy page. */
	if (cow_disable(pg))
		goto error1;

	unlockreg(preg->reg);
	return(0);

error1:
	unlockreg(preg->reg);
error0:
	return (-1);
}

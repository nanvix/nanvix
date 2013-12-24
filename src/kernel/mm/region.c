/*
 * Copyright (C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * mm/region.c - Memory region library implementation.
 */

#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/paging.h>
#include <nanvix/pm.h>
#include <nanvix/region.h>
#include <sys/types.h>
#include <sys/stat.h>

/* Memory region table. */
PRIVATE struct region regtab[NR_REGIONS];

/*
 * Locks a memory region.
 */
PUBLIC void lockreg(struct region *reg)
{
	/* Sleep until region is unlocked. */
	while (reg->flags & REGION_LOCKED)
		sleep(&reg->chain, PRIO_REGION);
	
	reg->flags |= REGION_LOCKED;
}

/*
 * Unlocks a memory region.
 */
PUBLIC void unlockreg(struct region *reg)
{
	reg->flags &= ~REGION_LOCKED;
	wakeup(&reg->chain);
}

/*
 * Allocates a memory region.
 */
PUBLIC struct region *allocreg(mode_t mode, size_t size, int flags)
{
	int i;              /* Loop index.        */
	int npages;         /* Number of pages.   */
	struct region *reg; /* Region.            */
	struct pte *pgtab;  /* Region page table. */

	#define MASK \
		(REGION_FREE|REGION_VALID|REGION_LOADING|REGION_DEMAND|REGION_LOCKED)
	
	/* Bad region size. */
	if (size > PGTAB_SIZE)
		return (NULL);
	
	/* Search for free region. */
	for (reg = &regtab[0]; reg < &regtab[NR_REGIONS]; reg++)
	{
		/* Found. */
		if (reg->flags & REGION_FREE)
			goto found;
	}

	kprintf("region table overflow");
	
	return (NULL);

found:

	pgtab = getkpg();
	
	/* Failed to allocate page table. */
	if (pgtab == NULL)
		return (NULL);

	size = ALIGN(size, PAGE_SIZE);
	npages = size/PAGE_SIZE;
	
	/* Mark pages as demand zero. */
	if (flags & REGION_DOWN)
	{
		npages = PAGE_SIZE/PTE_SIZE - npages;
		for (i = (PAGE_SIZE/PTE_SIZE - 1); i >= npages; i--)
			markpg(&pgtab[i], CLEAR);
	}
	
	else
	{
		for (i = 0; i < npages; i++)
			markpg(&pgtab[i], CLEAR);
	}
	
	
	/* Initialize region. */
	reg->flags = flags & ~MASK;
	reg->count = 0;
	reg->size = size;
	reg->pgtab = pgtab;
	reg->chain = NULL;
	reg->file.inode = NULL;
	reg->file.off = 0;
	reg->file.size = 0;
	reg->mode = mode & MAY_ALL;
	reg->cuid = curr_proc->uid;
	reg->cgid = curr_proc->gid;
	reg->uid = curr_proc->uid;
	reg->gid = curr_proc->gid;
	
	lockreg(reg);
	
	return (reg);
}

/*
 * Frees a memory region.
 */
PUBLIC void freereg(struct region *reg)
{
	int i;      /* Loop index.      */
	int npages; /* Number of pages. */
	
	/* Sticky region. */
	if (reg->flags & REGION_STICKY)
		return;
	
	/* Release region inode. */
	if (reg->file.inode != NULL)
		inode_put(reg->file.inode);
	
	/* Free region */
	npages = reg->size/PAGE_SIZE;
	for (i = 0; i < npages; i++)
		freeupg(&reg->pgtab[i]);
	putkpg(reg->pgtab);
	reg->flags = REGION_FREE;
}

/*
 * Attaches a memory region to a process.
 */
PUBLIC int attachreg
(struct process *proc, struct pregion *preg, addr_t addr, struct region *reg)
{
	int err;     /* Error?              */
	mode_t mode; /* Access permissions. */
	
	/* Process region is busy. */
	if (preg->reg != NULL)
		return (-1);
	
	/* Region grows down. */
	if (reg->flags & REGION_DOWN)
		addr &= PGTAB_MASK;
	
	/* Address must be page table aligned. */
	if (addr & ~PGTAB_MASK)
		return (-1);
		
	/* Process cannot grow more. */
	if (proc->size + reg->size > PROC_SIZE_MAX)
		return (-1);
	
	/* Non shared memory region is already attached to the process. */
	if (!(reg->flags & REGION_SHARED) && (reg->count > 0))
		return (-1);

	mode = accessreg(proc, reg);
	
	/* Attaching shared region, so check read/write permissions. */
	if (reg->flags & REGION_SHARED)
	{
		/* Do not have perssions to read/write region. */
		if (!(mode & (MAY_READ | MAY_WRITE)))
			return (-1);
	}

	err = mappgtab(proc->pgdir, addr, reg->pgtab, mode & MAY_WRITE);
	
	/* Failed to map page table. */
	if (err)
		return (-1);
	
	/* Attach region. */
	preg->start = addr;
	preg->reg = reg;
	reg->count++;
	reg->flags |= REGION_DEMAND;
	
	/* Increment process size. */
	proc->size += reg->size;
	
	return (0);
}

/*
 * Detaches a memory region from a process.
 */
PUBLIC void detachreg(struct process *proc, struct pregion *preg)
{
	struct region *reg;
	
	reg = preg->reg;
	
	/* Nothing to be done. */
	if (reg == NULL)
		return;
	
	lockreg(reg);
	
	/* Detach region. */
	umappgtab(proc->pgdir, preg->start);
	preg->reg = NULL;
	proc->size -= reg->size;
	
	if (reg->count > 0)
		reg->count--;
	
	if (reg->count == 0)
		freereg(reg);
	
	unlockreg(reg);
}

/*
 * Duplicates a memory region.
 */
PUBLIC struct region *dupreg(struct region *reg)
{
	int i;                  /* Loop index.        */
	struct region *new_reg; /* New memory region. */
	
	/* Shared region. */
	if (reg->flags & REGION_SHARED)
		return (reg);
	
	new_reg = allocreg(reg->flags, reg->mode, reg->size);
	
	/* Failed to allocate new region. */
	if (new_reg == NULL)
		return (NULL);
	
	/* Link all pages in the region. */
	for (i = 0; i < (int)reg->size/PAGE_SIZE; i++)
		linkupg(&reg->pgtab[i], &new_reg->pgtab[i]);
	
	/* Copy region fields. */
	new_reg->flags = reg->flags;
	if (reg->file.inode != NULL)
	{
		new_reg->file.inode = reg->file.inode;
		new_reg->file.off = reg->file.off;
		new_reg->file.size = reg->file.size;
		reg->file.inode->count++;
	}
	
	return (new_reg);
}

/*
 * Changes the size of a memory region.
 */
PUBLIC int growreg(struct process *proc, struct pregion *preg, ssize_t size)
{
	int i;
	int npages;
	struct region *reg;
	
	size = ALIGN(size, PAGE_SIZE);
	reg = preg->reg;
	
	/* Attached shared regions may not grow. */
	if (reg->flags & REGION_SHARED)
		return (-1);
	
	/* Region size is decreasing. */
	if (size < 0)
	{
		/* Region cannot have negative size. */
		if ((size_t)(-size) > reg->size)
			return (-1);

		npages = reg->size/PAGE_SIZE;
		
		/* Free pages. */
		for (i = (reg->size + size)/PAGE_SIZE; i < npages; i++)
			freeupg(&reg->pgtab[i]);
	}
	
	/* Region size is increasing. */
	else
	{
		/* Process cannot grow more. */
		if (proc->size + size > PROC_SIZE_MAX)
			return (-1);		
	}
	
	/* Change process and region sizse. */
	proc->size += size;
	reg->size += size;
	
	return (0);
}

/*
* Finds a memory region
*/
PUBLIC struct pregion *findreg(struct process *proc, addr_t addr)
{        
	int i;
	
	addr &= PGTAB_MASK;
	
	/* Find associated region. */
	for (i = 0; i < NR_PREGIONS; i++)
	{
		/* Found. */
		if (proc->pregs[i].reg != NULL)
		{
			if (addr >= proc->pregs[i].start)
			{
				if (addr < proc->pregs[i].start + proc->pregs[i].reg->size)
					return (&proc->pregs[i]);
			}
		}
	}

	return (NULL);
}

/*
 * Loads a portion of a file into a memory region.
 */
PUBLIC int loadreg(struct inode *inode, struct region *reg, off_t off, size_t size)
{
	int i;      /* Loop index.      */
	int npages; /* Number of pages. */
	
	reg->flags |= REGION_LOADING;
	reg->file.inode = inode;
	reg->file.off = off;
	reg->file.size = size;
	reg->file.inode->count++;
	
	npages = ALIGN(size, PAGE_SIZE) >> PAGE_SHIFT;
	
	/* Mark pages as demand fill. */
	for (i = 0; i < npages; i++)
		markpg(&reg->pgtab[i], FILL);
	
	return (0);
}

/*
 * Initializes memory regions.
 */
PUBLIC void initreg(void)
{
	struct region *reg;
	
	/* Initialize memory region table. */
	for (reg = &regtab[0]; reg < &regtab[NR_REGIONS]; reg++)
		reg->flags = REGION_FREE;
	
	kprintf("mm: %d regions in memory regions table", NR_REGIONS);
}

/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * region.c - Memory regions.
 */


#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/paging.h>
#include <nanvix/pm.h>
#include <nanvix/region.h>
#include <sys/types.h>
#include <stat.h>

/* Number of memory regions. */
#define NR_REGIONS (PROC_MAX*NR_PREGIONS)

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
PUBLIC struct region *allocreg(int shared, mode_t mode, size_t size)
{
	int i;
	struct pte *pgtab;
	
	/* Search for free region. */
	for (i = 0; i < NR_REGIONS; i++)
	{
		/* Found. */
		if (regtab[i].flags & REGION_FREE)
			goto found;
	}

	kprintf("region table overflow");
	
	return (NULL);

found:

	mode &= (S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH);
	
	pgtab = getkpg();
	
	/* Failed to allocate page table. */
	if (pgtab == NULL)
		return (NULL);
	
	/* Initialize region. */
	regtab[i].flags |= ((shared) ? REGION_SHARED : 0) & ~REGION_FREE;
	regtab[i].size = size;
	regtab[i].mode = mode;
	regtab[i].cuid = curr_proc->uid;
	regtab[i].cgid = curr_proc->gid;
	regtab[i].cpid = curr_proc->pid;
	regtab[i].uid = curr_proc->uid;
	regtab[i].gid = curr_proc->gid;
	regtab[i].pgtab = pgtab;
	
	lockreg(&regtab[i]);
	
	return (&regtab[i]);
}

/*
 * Frees a memory region.
 */
PUBLIC void freereg(struct region *reg)
{
	int i;
	
	/* Nothing to be done. */
	if (reg->count > 0)
	{
		unlockreg(reg);
		return;
	}
	
	/* Free underlying pages. */
	for (i = 0; i < (int)reg->size/PAGE_SIZE; i++)
		freeupg(&reg->pgtab[i]);
	
	/* Free underlying page table. */
	putkpg(reg->pgtab);
	
	/* Clear region fields. */
	reg->flags = 0 | REGION_FREE;
	reg->count = 0;
	reg->size = 0;
	reg->nvalid = 0;
	reg->mode = 0;
	reg->cuid = 0;
	reg->cgid = 0;
	reg->cpid = 0;
	reg->uid = 0;
	reg->gid = 0;
	reg->pgtab = NULL;
	reg->chain = NULL;
}

/*
 * Checks if a process has read/write permissions to a memory region.
 */
PUBLIC int accessreg(struct process *proc, struct region *reg)
{
	int rw;
	
	/* Same user ID. */
	if (proc->uid == reg->uid)
	{
		/* Write permissions. */
		if (reg->mode & S_IWUSR)
			rw = 1;
	
		/* Read permissions. */
		if (reg->mode & S_IRUSR)
			rw = 0;
			
		/* Cannot read nor write. */
		else
			return (-1);
	}
		
	/* Same group ID. */
	else if (proc->gid == reg->gid)
	{
		/* Write permissions. */
		if (reg->mode & S_IWGRP)
				rw = 1;
			
		/* Read permissions. */
		if (reg->mode & S_IRGRP)
			rw = 0;
			
		/* Cannot read nor write. */
		else
			return (-1);
	}
		
	/* Others. */
	else 
	{
		/* Write permissions. */
		if (reg->mode & S_IWOTH)
			rw = 1;
			
		/* Read permissions. */
		if (reg->mode & S_IROTH)
			rw = 0;
		
		/* Cannot read nor write. */
		else
			return (-1);
	}
	
	return (rw);
}

/*
 * Attaches a memory region to a process.
 */
PUBLIC int attachreg(struct process *proc, addr_t addr, int type, struct region *reg)
{
	int i;
	int rw;
	int err;
	
	/* Address must be page table aligned. */
	if (addr & ~PGTAB_MASK)
		return (-1);
	
	/* Process cannot grow more. */
	if (proc->size + reg->size > PROC_SIZE_MAX)
		return (-1);
		
	/* Non shared memory region is already attached to the process. */
	if (!(reg->flags & REGION_SHARED) && (reg->count == 1))
		return (-1);
	
	/* Search for free process memory region. */
	for (i = 0; i < NR_PREGIONS; i++)
	{
		/* Found. */
		if (proc->pregs[i].type == PREGION_UNUSED)
			goto found;
	}

	kprintf("process region table overflow");
	
	return (-1);

found:
	
	rw = 1;
		
	/* Attaching shared region, so check read/write permissions. */
	if (reg->flags & REGION_SHARED)
	{
		rw = accessreg(proc, reg);
		
		/* Do not have perssions to read/write region. */
		if (rw < 0)
			return (-1);
	}

	err = mappgtab(proc->pgdir, addr, reg->pgtab, rw);
	
	/* Failed to map page table. */
	if (err)
		return (-1);
	
	/* Attach region. */
	proc->pregs[i].type = type;
	proc->pregs[i].start = addr;
	proc->pregs[i].reg = reg;
	reg->count++;
	reg->flags |= REGION_DEMAND;
	
	/* Increment process size. */
	proc->size += reg->size;
	
	return (0);
}

/*
 * Detaches a memory region from a process.
 */
PUBLIC struct region *detachreg(struct process *proc, struct pregion *preg)
{
	struct region *reg;
	
	reg = preg->reg;
	
	/* Detach region. */
	umappgtab(proc->pgdir, preg->start);
	reg->count--;
			
	/* Detaches region. */
	preg->type = PREGION_UNUSED;
	preg->start = 0;
	preg->reg = NULL;
		
	/* Decrement process size. */
	proc->size -= reg->size;
	
	return (reg);
}

/*
 * Duplicates a memoru region.
 */
PUBLIC struct region *dupreg(struct region *reg)
{
	int i;
	
	struct region *new_reg;
	
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
	new_reg->nvalid = reg->nvalid;
	
	return (reg);
}

/*
 * Changes the size of a memory region.
 */
PUBLIC int growreg(struct process *proc, struct pregion *preg, ssize_t size)
{
	int i;
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
		
		/* Free pages. */
		for (i = (reg->size + size)/PAGE_SIZE; i < (int)reg->size/PAGE_SIZE;i++)
		{
			freeupg(&reg->pgtab[i]);
			reg->nvalid--;
		}
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
	
	/* Find associated region. */
	for (i = 0; i < NR_PREGIONS; i++)
	{
		/* Found. */
		if (proc->pregs[i].type != PREGION_UNUSED)
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
 * Initializes memory regions.
 */
PUBLIC void initreg()
{
	int i;
	
	/* Initialize memory region table. */
	for (i = 0; i < NR_REGIONS; i++)
	{
		regtab[i].flags = REGION_FREE;
		regtab[i].count = 0;
		regtab[i].size = 0;
		regtab[i].nvalid = 0;
		regtab[i].mode = 0;
		regtab[i].cuid = 0;
		regtab[i].cgid = 0;
		regtab[i].cpid = 0;
		regtab[i].uid = 0;
		regtab[i].gid = 0;
		regtab[i].pgtab = NULL;
		regtab[i].chain = NULL;
	}
}

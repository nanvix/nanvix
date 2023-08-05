/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2016 Davidson Francis <davidsondfgl@gmail.com>
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
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <nanvix/region.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <errno.h>
#include "mm.h"

/**
 * @brief Memory region table.
 */
PRIVATE struct region regtab[NR_REGIONS];

/**
 * @brief Expands a memory region.
 *
 * @param proc Process who owns the memory region.
 * @param reg  Memory region that shall be expanded.
 * @param size Size in bytes to be added to the memory region.
 */
PRIVATE int expand(struct process *proc, struct region *reg, size_t size)
{
	unsigned i, j;        /* Loop indexes.                  */
	unsigned npages;      /* Number of pages in the region. */
	struct pregion *preg; /* Working process region.        */

	size = ALIGN(size, PAGE_SIZE);

	/* Region too big. */
	if (reg->size + size > REGION_SIZE)
		return (-1);

	preg = reg->preg;
	npages = size >> PAGE_SHIFT;

	/* Expand downwards. */
	if (reg->flags & REGION_DOWNWARDS)
	{
		/* Allocate first page table. */
		if (reg->size == 0)
		{
			reg->pgtab[REGION_PGTABS - 1] = getkpg(1);
			if (reg->pgtab[REGION_PGTABS - 1] == NULL)
				return (-1);
		}

		i = REGION_PGTABS - (reg->size >> PGTAB_SHIFT) - 1;
		j = PAGE_SIZE/PTE_SIZE -
				(((PAGE_MASK^PGTAB_MASK) & reg->size) >> PAGE_SHIFT);

		if (reg->size != 0 && j == (PAGE_SIZE / PTE_SIZE) - 1)
		{
			reg->pgtab[i] = getkpg(1);
			if (reg->pgtab[i] == NULL)
				return (-1);

			/* Map page table. */
			if (proc != NULL)
				mappgtab(proc, preg->start + reg->size, reg->pgtab[i]);
		}

		/* Mark pages as demand zero. */
		while (npages > 0)
		{
			/* Create new page table. */
			if (j == 0)
			{
				i--;
				j = PAGE_SIZE/PTE_SIZE;

				reg->pgtab[i] = getkpg(1);
				if (reg->pgtab[i] == NULL)
					return (-1);

				/* Map page table. */
				if (proc != NULL)
					mappgtab(proc, preg->start - reg->size, reg->pgtab[i]);

				continue;
			}

			j--;
			npages--;
			reg->size += PAGE_SIZE;

			markpg(&reg->pgtab[i][j], PAGE_ZERO);
		}
	}

	/* Expand upwards. */
	else
	{
		/* Allocate first page table. */
		if (reg->size == 0)
		{
			reg->pgtab[0] = getkpg(1);
			if (reg->pgtab[0] == NULL)
				return (-1);
		}

		i = reg->size >> PGTAB_SHIFT;
		j = ((PAGE_MASK^PGTAB_MASK) & reg->size) >> PAGE_SHIFT;

		if (reg->size != 0 && j == 0)
		{
			reg->pgtab[i] = getkpg(1);
			if (reg->pgtab[i] == NULL)
				return (-1);

			/* Map page table. */
			if (proc != NULL)
				mappgtab(proc, preg->start + reg->size, reg->pgtab[i]);
		}

		/* Mark pages as demand zero. */
		while (npages > 0)
		{
			/* Reset. */
			if (j == PAGE_SIZE/PTE_SIZE)
			{
				i++;
				j = 0;

				reg->pgtab[i] = getkpg(1);
				if (reg->pgtab[i] == NULL)
					return (-1);

				/* Map page table. */
				if (proc != NULL)
					mappgtab(proc, preg->start + reg->size, reg->pgtab[i]);

				continue;
			}

			markpg(&reg->pgtab[i][j], PAGE_ZERO);

			j++;
			npages--;
			reg->size += PAGE_SIZE;
		}
	}

	return (0);
}

/**
 * @brief Contracts a memory region.
 *
 * @param proc Process who owns the memory region.
 * @param reg  Memory region that shall be contracted.
 * @param size Size in bytes to be removed from the memory region.
 *
 * @returns Zero upon success, and non-zero otherwise.
 */
PRIVATE int contract(struct process *proc, struct region *reg, size_t size)
{
	unsigned i, j;        /* Loop indexes.                  */
	unsigned npages;      /* Number of pages in the region. */
	struct pregion *preg; /* Working process region.        */

	size = ALIGN(size, PAGE_SIZE);

	/* Region cannot have negative size. */
	if (size > reg->size)
		return (-1);

	preg = reg->preg;
	npages = reg->size >> PAGE_SHIFT;

	/* Contract downwards. */
	if (reg->flags & REGION_DOWNWARDS)
	{
		i = REGION_PGTABS - (reg->size >> PGTAB_SHIFT) - 1;
		j = PAGE_SIZE/PTE_SIZE -
				(((PAGE_MASK^PGTAB_MASK) & reg->size) >> PAGE_SHIFT);

		/* Mark pages as demand zero. */
		while (npages > 0)
		{
			/* Create new page table. */
			if (j == PAGE_SIZE/PTE_SIZE)
			{
				/* Unmap page table. */
				if (proc != NULL)
					umappgtab(proc, preg->start - reg->size);
				putkpg(reg->pgtab[i]);
				reg->pgtab[i] = NULL;

				i++;
				j = 0;

				continue;
			}

			freeupg(&reg->pgtab[i][j]);

			j++;
			npages--;
			reg->size -= PAGE_SIZE;

		}
	}

	/* Contract upwards. */
	else
	{
		i = reg->size >> PGTAB_SHIFT;
		j = ((PAGE_MASK^PGTAB_MASK) & reg->size) >> PAGE_SHIFT;

		/* Mark pages as demand zero. */
		while (npages > 0)
		{
			/* Reset. */
			if (j == 0)
			{
				/* Unmap page table. */
				if (proc != NULL)
					umappgtab(proc, preg->start - reg->size);
				putkpg(reg->pgtab[i]);
				reg->pgtab[i] = NULL;

				i--;
				j = PAGE_SIZE/PTE_SIZE;

				continue;
			}

			j--;
			npages--;
			reg->size -= PAGE_SIZE;

			freeupg(&reg->pgtab[i][j]);
		}
	}

	return (0);
}

/**
 * @brief Locks a memory region.
 *
 * @param reg Memory region that shall be locked.
 */
PUBLIC void lockreg(struct region *reg)
{
	/* Sleep until region is unlocked. */
	while (reg->flags & REGION_LOCKED)
		sleep(&reg->chain, PRIO_REGION);

	reg->flags |= REGION_LOCKED;
}

/**
 * @brief Unlocks a memory region.
 *
 * @param reg Memory region to be unlocked.
 */
PUBLIC void unlockreg(struct region *reg)
{
	reg->flags &= ~REGION_LOCKED;
	wakeup(&reg->chain);
}

/**
 * @brief Allocates a memory region.
 *
 * @param mode  Access permissions.
 * @param size  Size in bytes.
 * @param flags Memory region flags.
 *
 * @returns Upon success a pointer to a memory region is returned. Upon failure,
 *          a NULL pointer is returned instead.
 */
PUBLIC struct region *allocreg(mode_t mode, size_t size, int flags)
{
	unsigned i;         /* Loop index. */
	struct region *reg; /* Region.     */

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

	/* Initialize region. */
	reg->flags = flags & ~(REGION_FREE | REGION_LOCKED);
	reg->count = 0;
	reg->size = 0;
	reg->chain = NULL;
	reg->file.inode = NULL;
	reg->file.off = 0;
	reg->file.size = 0;
	reg->mode = mode;
	reg->cuid = curr_proc->uid;
	reg->cgid = curr_proc->gid;
	reg->uid = curr_proc->uid;
	reg->gid = curr_proc->gid;
	for (i = 0; i < REGION_PGTABS; i++)
		reg->pgtab[i] = NULL;

	/* Expand region. */
	if (expand(NULL, reg, size))
	{
		reg->flags = REGION_FREE;
		return (NULL);
	}

	lockreg(reg);

	return (reg);
}

/**
 * @brief Frees a memory region.
 *
 * @param reg Memory region that shall be freed.
 */
PUBLIC void freereg(struct region *reg)
{
	unsigned i, j;

	/* Sticky region. */
	if (reg->flags & REGION_STICKY)
		return;

	/* Release region inode. */
	if (reg->file.inode != NULL)
		inode_put(reg->file.inode);

	/* Free underlying page tables. */
	for (i = 0; i < REGION_PGTABS; i++)
	{
		/* Skip invalid page tables. */
		if (reg->pgtab[i] == NULL)
			continue;

		/* Free underlying pages. */
		for (j = 0; j < PAGE_SIZE/PTE_SIZE; j++)
			freeupg(&reg->pgtab[i][j]);
		putkpg(reg->pgtab[i]);
	}

	reg->flags = REGION_FREE;
}

/**
 * @brief Edits access permissions on a memory region.
 *
 * @param reg  Memory region to be edited.
 * @param uid  New user ID.
 * @param gid  New group ID.
 * @param mode New access permissions.
 *
 * @returns Zero upon success, and non-zero otherwise.
 */
PUBLIC int editreg(struct region *reg, uid_t uid, gid_t gid, mode_t mode)
{
	reg->uid = uid;
	reg->gid = gid;
	reg->mode = mode;

	return (0);
}

/**
 * @brief Attaches a memory region to a process.
 *
 * @param proc  Process where the memory region shall be attached.
 * @param preg  Process memory region where the memory shall be attached.
 * @param start Address where the memory region shall be attached.
 * @param reg   Memory region to be attached.
 *
 * @returns Zero upon success, and non-zero otherwise.
 */
PUBLIC int attachreg
(struct process *proc, struct pregion *preg, addr_t start, struct region *reg)
{
	addr_t addr; /* Working address. */
	unsigned i;  /* Loop index.      */

	/* Process region is busy. */
	if (preg->reg != NULL)
		return (-1);

	/* Bad address. */
	if ((start < UBASE_VIRT) || (start >= KBASE_VIRT))
	{
		curr_proc->errno = -EFAULT;
		return (-1);
	}

	/* Region grows downwards. */
	if (reg->flags & REGION_DOWNWARDS)
	{
		/* Address is not properly aligned. */
		if ((start & ~PGTAB_MASK) != ~PGTAB_MASK)
			return (-1);
	}

	/* Region grows upwards. */
	else
	{
		/* Address is not properly aligned. */
		if (start & ~PGTAB_MASK)
			return (-1);
	}

	/* Process cannot grow more. */
	if (proc->size + reg->size > PROC_SIZE_MAX)
		return (-1);

	/* Attaching shared region. */
	if (reg->flags & REGION_SHARED)
	{
		/* Do not have perssions to read/write region. */
		if (!(accessreg(proc, reg) & (MAY_READ | MAY_WRITE)))
			return (-1);
	}

	/* Attaching private region. */
	else
	{
		/* Already attached to the process. */
		if (reg->count > 0)
			return (-1);
	}

	/* Map page tables. */
	addr = start;
	if (reg->flags & REGION_DOWNWARDS)
	{
		for (i = REGION_PGTABS; i > 0 ; i--)
		{
			/* Map only valid page tables. */
			if (reg->pgtab[i - 1] != NULL)
				mappgtab(proc, addr, reg->pgtab[i - 1]);
			addr -= PGTAB_SIZE;
		}
	}
	else
	{
		for (i = 0; i < REGION_PGTABS; i++)
		{
			/* Map only valid page tables. */
			if (reg->pgtab[i] != NULL)
				mappgtab(proc, addr, reg->pgtab[i]);
			addr += PGTAB_SIZE;
		}
	}

	/* Attach region. */
	preg->start = start;
	preg->reg = reg;
	reg->count++;
	reg->preg = preg;
	proc->size += reg->size;

	return (0);
}

/**
 * @brief Detaches a memory region from a process.
 *
 * @param proc Process where the memory region is attached.
 * @param preg Process memory region to be used.
 */
PUBLIC void detachreg(struct process *proc, struct pregion *preg)
{
	unsigned i;         /* Loop index.            */
	addr_t addr;        /* Working address.       */
	struct region *reg; /* Working memory region. */

	/* Nothing to be done. */
	if ((reg = preg->reg) == NULL)
		return;

	lockreg(reg);

	/* Detach region. */
	addr = preg->start;
	if (reg->flags & REGION_DOWNWARDS)
	{
		for (i = REGION_PGTABS; i > 0 ; i--)
		{
			/* Unmap only valid page tables. */
			if (reg->pgtab[i - 1] != NULL)
				umappgtab(proc, addr);
			addr -= PGTAB_SIZE;
		}
	}
	else
	{
		for (i = 0; i < REGION_PGTABS; i++)
		{
			/* Unmap only valid page tables. */
			if (reg->pgtab[i] != NULL)
				umappgtab(proc, addr);
			addr += PGTAB_SIZE;
		}
	}
	preg->reg = NULL;
	proc->size -= reg->size;
	if (--reg->count < 0)
		kpanic("mm: detaching memory region twice");

	unlockreg(reg);

	/* Free region. */
	if (reg->count == 0)
		freereg(reg);
}

/**
 * @brief Duplicates a memory region.
 *
 * @param reg Memory region that shall be duplicated.
 *
 * @returns Upon success a pointer to the (duplicated) memory region is
 *          returned. Upon failure, a NULL pointer is returned instead.
 */
PUBLIC struct region *dupreg(struct region *reg)
{
	unsigned i, j;          /* Loop index.        */
	struct region *new_reg; /* New memory region. */

	/* Shared region. */
	if (reg->flags & REGION_SHARED)
		return (reg);

	/* Failed to allocate new region. */
	if ((new_reg = allocreg(reg->mode, reg->size, reg->flags)) == NULL)
		return (NULL);

	/* Link underlying page tables. */
	for (i = 0; i < REGION_PGTABS; i++)
	{
		/* Skip invalid page tables. */
		if (reg->pgtab[i] == NULL)
			continue;

		/* Link underlying pages. */
		for (j = 0; j < PAGE_SIZE/PTE_SIZE; j++)
			linkupg(&reg->pgtab[i][j], &new_reg->pgtab[i][j]);
	}

	/* Copy region fields. */
	if (reg->file.inode != NULL)
	{
		new_reg->file.inode = reg->file.inode;
		new_reg->file.off = reg->file.off;
		new_reg->file.size = reg->file.size;
		reg->file.inode->count++;
	}

	return (new_reg);
}

/**
 * @brief Changes the size of memory region.
 *
 * @param proc Process where the memory region is attached to.
 * @param preg Process region where the memory region is attached.
 * @param size Increment/decrement in bytes.
 *
 * @returns Zero upon success, and non-zero otherwise.
 */
PUBLIC int growreg(struct process *proc, struct pregion *preg, ssize_t size)
{
	struct region *reg;

	/* Attached shared regions may not grow. */
	if ((reg = preg->reg)->flags & REGION_SHARED)
		return (-EINVAL);

	/* Region cannot grow. */
	if (!(reg->flags & (REGION_DOWNWARDS | REGION_UPWARDS)))
		return (-EINVAL);

	/* Contract region */
	if (size < 0)
		contract(proc, reg, -size);

	/* Expand region. */
	else
	{
		/* Process cannot grow more. */
		if (proc->size + size > PROC_SIZE_MAX)
			return (-ENOMEM);

		/* Failed to expand region.  */
		if (expand(proc, reg, size))
			return (-ENOMEM);
	}

	/* Change process and region sizse. */
	proc->size += size;

	return (0);
}

/**
 * @brief Finds a memory region.
 *
 * @param proc Process where the memory region shall be searched.
 * @param addr Address to be queried.
 *
 * @returns Upon success a pointer to the process memory region requested is
 *          returned. Upon failure, a NULL pointer is returned instead.
 */
PUBLIC struct pregion *findreg(struct process *proc, addr_t addr)
{
	struct region *reg;   /* Working memory region.  */
	struct pregion *preg; /* Working process region. */

	/* Find associated region. */
	for (preg = &proc->pregs[0]; preg < &proc->pregs[NR_PREGIONS]; preg++)
	{
		/* Skip invalid regions. */
		if ((reg = preg->reg) == NULL)
			continue;

		/* Region grows downwards. */
		if (reg->flags & REGION_DOWNWARDS)
		{
			if (addr <= preg->start)
			{
				if (addr >= preg->start - REGION_SIZE)
					return (preg);
			}
		}

		/* Region grows upwards. */
		else
		{
			if (addr >= preg->start)
			{
				if (addr < preg->start + REGION_SIZE)
					return (preg);
			}
		}
	}

	return (NULL);
}

/**
 * @brief Loads a portion of a file into a memory region.
 *
 * @param inode Inode associated to the file.
 * @param reg   Memory region in which the file will be loaded.
 * @param off   File offset.
 * @param size  Number of bytes to be loaded.
 *
 * @returns Zero upon success, and non-zero otherwise.
 */
PUBLIC int loadreg
(struct inode *inode, struct region *reg, off_t off, size_t size)
{
	unsigned i, j;   /* Loop indexes.    */
	unsigned npages; /* Number of pages. */

	reg->file.inode = inode;
	reg->file.off = off;
	reg->file.size = size;
	reg->file.inode->count++;

	npages = ALIGN(size, PAGE_SIZE) >> PAGE_SHIFT;

	/* Mark pages as demand fill. */
	if (reg->flags & REGION_DOWNWARDS)
	{
		i = REGION_PGTABS - 1;
		j = PAGE_SIZE/PTE_SIZE;
		while (npages > 0)
		{
			/* Reset. */
			if (j == 0)
			{
				i--;
				j = PAGE_SIZE/PTE_SIZE;
				continue;
			}

			j--;
			npages--;

			markpg(&reg->pgtab[i][j], PAGE_FILL);
		}
	}
	else
	{
		i = 0;
		j = 0;
		while (npages > 0)
		{
			/* Reset. */
			if (j == PAGE_SIZE/PTE_SIZE)
			{
				i++;
				j = 0;
				continue;
			}

			markpg(&reg->pgtab[i][j], PAGE_FILL);

			j++;
			npages--;
		}
	}

	return (0);
}

/**
 * @brief Initializes memory regions.
 */
PUBLIC void initreg(void)
{
	struct region *reg;

	/* Initialize memory region table. */
	for (reg = &regtab[0]; reg < &regtab[NR_REGIONS]; reg++)
		reg->flags = REGION_FREE;

	kprintf("mm: %d regions in memory regions table", NR_REGIONS);
}

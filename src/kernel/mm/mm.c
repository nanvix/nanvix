/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <nanvix/region.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>

/**
 * @brief Initializes the memory system.
 */
PUBLIC void mm_init(void)
{
	initreg();
}

/**
 * @brief Checks access permissions to a memory area.
 * 
 * @param addr Address to be checked.
 * @param size Size of memory area.
 * @param mask Access permissions mask.
 * 
 * @returns Zero if access is authorized, and non-zero otherwise.
 */
PUBLIC int chkmem(const void *addr, size_t size, mode_t mask)
{
	int ret;              /* Return value.           */
	struct region *reg;   /* Working memory region.  */
	struct pregion *preg; /* Working process region. */
	
	/* Get associated process memory region. */
	if ((preg = findreg(curr_proc, ADDR(addr))) == NULL)
		return (-1);
	
	lockreg(reg = preg->reg);
	
	/* Not allowed. */
	if (!(accessreg(curr_proc, reg) & mask))
	{
		unlockreg(reg);
		return (-1);
	}
		
	ret = withinreg(preg, ADDR(addr));
	ret &= withinreg(preg, ADDR(addr) + size);

	unlockreg(reg);
	
	return (ret);
}

/**
 * @brief Fetches a byte from user address space.
 * 
 * @param addr Address where the byte should be fetched.
 * 
 * @returns Upon successful completion the byte fetched is returned 
 *          (casted to int). Upon failure, -1 is returned instead. 
 */
PUBLIC int fubyte(const void *addr)
{	
	int byte;            /* User byte.               */
	struct pregion *preg; /* Working process region. */
	
	/* Kernel address space. */
	if (((addr_t)addr < UBASE_VIRT) || ((addr_t)addr >= KBASE_VIRT))
	{
		if (KERNEL_RUNNING(curr_proc))
			return (*((char *)addr));
		
		return (-1);
	}
	
	/* Get associated process region. */
	if ((preg = findreg(curr_proc, (addr_t)addr)) == NULL)
		return (-1);
	
	byte = (withinreg(preg, ADDR(addr))) ? (*((char *)addr)) : -1;
	
	return (byte);
}

/**
 * @brief Fetches a double word (4 bytes) from user address space.
 * 
 * @param addr Address where the byte should be fetched.
 * 
 * @returns Upon successful completion the byte fetched is returned (casted to
 *          int). Upon failure, -1 is returned instead.
 */
PUBLIC int fudword(const void *addr)
{	
	int dword;            /* User double word.       */
	struct pregion *preg; /* Working process region. */
	
	/* Kernel address space. */
	if (((addr_t)addr < UBASE_VIRT) || ((addr_t)addr >= KBASE_VIRT))
	{
		if (KERNEL_RUNNING(curr_proc))
			return (*((int *)addr));
		
		return (-1);
	}
	
	/* Get associated process region. */
	if ((preg = findreg(curr_proc, (addr_t)addr)) == NULL)
		return (-1);
	
	dword = (withinreg(preg, ADDR(addr))) ? (*((int *)addr)) : -1;
	
	return (dword);
}

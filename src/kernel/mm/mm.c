/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * mm/mm.c - Memory subsystem module.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <nanvix/region.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>

/*
 * Initializes the memory system module.
 */
PUBLIC void mm_init(void)
{
	/* Initialize memory regions. */
	initreg();
}

/*
 * Checks access permissions to a memory area.
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
		
	ret = withinreg(reg, ADDR(addr));
	ret |= withinreg(reg, ADDR(addr) + size);

	unlockreg(reg);
	
	return (ret);
}

/*
 * Fetches a byte from user address space.
 */
PUBLIC int fubyte(const void *addr)
{	
	int byte;            /* User byte.               */
	struct pregion *preg; /* Working process region. */
	
	/* Get associated process region. */
	if ((preg = findreg(curr_proc, (addr_t)addr)) == NULL)
		return (-1);
	
	byte = (withinreg(preg->reg, addr)) ? (*((char *)addr)) : -1;
	
	return (byte);
}

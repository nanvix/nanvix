/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * mm/mm.c - Memory system module.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <nanvix/region.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <stdarg.h>

/*
 * Initializes the memory system module.
 */
PUBLIC void mm_init()
{
	/* Initialize memory regions. */
	initreg();
}

/*
 * Checks access permissions to a memory area.
 */
PUBLIC int chkmem(addr_t addr, int type, ...)
{
	size_t size;
	int writable;
	va_list args;
	struct pregion *preg;
	
	preg = findreg(curr_proc, addr);
	
	/* Memory area not attached to user address space. */
	if (preg == NULL)
		goto out0;
	
	lockreg(preg->reg);
	
	/* Check a chunk of memory. */
	if (type == CHK_CHUNK)
	{
		va_start(args, writable);
		writable = va_arg(args, size_t);
		size = va_arg(args, size_t);
		va_end(args);
		
		if ((addr + size) < (preg->start + preg->reg->size))
			goto out1;
	}
	
	/* Check a function area of memory. */
	else
		writable = 0;
	
	/* No read/write permission. */	
	if (writable > accessreg(curr_proc, preg->reg))
		goto out1;
	
	unlockreg(preg->reg);
	
	return (1);

out1:
	unlockreg(preg->reg);
out0:
	return (0);
}

/*
 * Fetches a byte from user addresss space.
 */
PUBLIC int fubyte(void *addr)
{	
	char byte;
	
	if ((addr_t)addr >= UBASE_VIRT)
	{
		if ((addr_t)addr < KBASE_VIRT)
		{
			if (ksetjmp(&curr_proc->kenv) != 0)
				return (-1);
			
			byte = (*((char *)addr));
			
			kusetjmp(&curr_proc->kenv);
			
			return ((int)byte);
		}
	}
	
	return (-1);
}

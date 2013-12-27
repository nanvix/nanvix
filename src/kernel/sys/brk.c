/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * brk.c - brk() system call implementation.
 * 
 * FIXME: return value.
 */

#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <nanvix/region.h>
#include <errno.h>

/*
 * Changes process' breakpoint value.
 */
PUBLIC int sys_brk(void *addr)
{
	int ret;
	struct pregion *preg;
	
	ret = 0;
	
	/* Invalid address. */
	if ((addr_t)addr < UHEAP_ADDR)
		return (-EINVAL);
	
	preg = findreg(curr_proc, (addr_t)addr);
	
	/* Invalid address. */
	if (preg != HEAP(curr_proc))
		return (-EFAULT);
	
	lockreg(preg->reg);
	
	ret = growreg(curr_proc, preg, (ssize_t)((addr_t)(addr) - preg->start));
	
	unlockreg(preg->reg);
	
	return ((ret) ? -EAGAIN : 0);
}

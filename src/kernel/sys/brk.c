/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/brk.c - brk() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <nanvix/region.h>
#include <errno.h>
#include <nanvix/klib.h>
/*
 * Changes process' breakpoint value.
 */
PUBLIC int sys_brk(void *addr)
{
	ssize_t size;         /* Increment size.     */
	struct pregion *heap; /* Heap process region.*/
	
	kprintf("brk(%d)", curr_proc->pid);
	
	heap = findreg(curr_proc, (addr_t)addr);
	
	/* Bad address. */
	if (heap != HEAP(curr_proc))
		return (-EFAULT);
	
	size = (addr_t)addr - (heap->start + heap->reg->size);
	
	/*
	 * There is no need to lock the heap region
	 * since it is a private region.
	 */
	
	return (growreg(curr_proc, heap, size));
}

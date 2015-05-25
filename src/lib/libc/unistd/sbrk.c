/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * unistd/sbrk.c - sbrk() implementation.
 */

#include <nanvix/mm.h>
#include <sys/types.h>
#include <stdlib.h>
#include <unistd.h>

/* Current breakpoint value. */
static void *current_brk = (void *)UHEAP_ADDR;

/*
 * Changes process's breakpoint value.
 */
void *sbrk(size_t size)
{
	void *old_brk;
	
	size = (((size) + (PAGE_SIZE - 1)) & ~(PAGE_SIZE - 1));
	
	old_brk = current_brk;
	current_brk = (void *)((unsigned)old_brk + size);
	if (brk(current_brk))
	{
		current_brk = old_brk;
		return ((void *) -1);
	}
	
	return (old_brk);
}

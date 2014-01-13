/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * unistd/sbrk.c - sbrk() implementation.
 */

#include <sys/types.h>
#include <stdlib.h>
#include <unistd.h>

/* Current breakpoint value. */
static void *current_brk = (void *)0xa0000000;

/*
 * Changes process's breakpoint value.
 */
void *sbrk(size_t size)
{
	void *old_brk;
	
	old_brk = current_brk;
	current_brk = (void *)((unsigned)old_brk + size);
	if (brk(current_brk))
		current_brk = old_brk;
	return (old_brk);
}

/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * unistd/brk.c - brk() system call.
 */

#include <nanvix/syscall.h>
#include <errno.h>

/*
 * Changes process' breakpoint value.
 */
int brk(void *addr)
{
	int ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_brk),
		  "b" (addr)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return (ret);
}

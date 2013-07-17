/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/wait/wait.c - wait() system call.
 */

#include <nanvix/syscall.h>
#include <sys/types.h>
#include <errno.h>

pid_t wait(int *stat_loc)
{
	pid_t pid;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (pid)
		: "0" (NR_wait),
		  "b" (stat_loc)
	);
	
	/* Error. */
	if (pid < 0)
	{
		errno = -pid;
		return (-1);
	}
	
	return (pid);
}


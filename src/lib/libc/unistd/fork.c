/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * unistd/fork.c  fork() system call.
 */

#include <nanvix/syscall.h>
#include <errno.h>

/*
 * Creates a new process.
 */
pid_t fork(void)
{
	pid_t pid;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (pid)
		: "0" (NR_fork)
	);
	
	/* Error. */
	if (pid < 0)
	{
		errno = -pid;
		return (-1);
	}
	
	return (pid);
}

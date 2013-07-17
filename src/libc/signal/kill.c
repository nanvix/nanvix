/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * signal/kill.c - kill() system call.
 */

#include <nanvix/syscall.h>
#include <errno.h>
#include <signal.h>

/*
 * Sends a signal to a process.
 */
int kill(pid_t pid, int sig)
{
	int ret;

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_kill),
		  "b" (pid),
		  "c" (sig)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}

	return (0);
}

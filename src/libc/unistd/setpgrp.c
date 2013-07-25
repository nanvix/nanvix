/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <unistd/setpgrp.c> - setpgrp() system call.
 */

#include <nanvix/syscall.h>
#include <sys/types.h>
#include <errno.h>

/*
 * Sets the process group ID.
 */
pid_t setpgrp(void)
{
	pid_t pid;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (pid)
		: "0" (NR_setpgrp)
	);
	
	/* Error. */
	if (pid < 0)
	{
		errno = -pid;
		return (-1);
	}
	
	return (pid);
}

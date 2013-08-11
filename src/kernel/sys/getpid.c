/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/getpid.c> - getpid() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <sys/types.h>

/*
 * Gets the process ID of the calling process.
 */
PUBLIC pid_t sys_getpid(void)
{
	return (curr_proc->pid);
}

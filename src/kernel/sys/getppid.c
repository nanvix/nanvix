/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * getppid.c - getppid() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <sys/types.h>

/*
 * Gets the parent process ID of the calling process.
 */
PUBLIC pid_t sys_getppid()
{
	return (curr_proc->father->pid);
}

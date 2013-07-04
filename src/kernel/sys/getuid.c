/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * getuid.c - getuid() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <sys/types.h>

/*
 * Gets the real user ID of the calling process.
 */
PUBLIC pid_t sys_getuid()
{
	return (curr_proc->uid);
}

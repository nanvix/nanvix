/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * geteuid.c - geteuid() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <sys/types.h>

/*
 *  Gets the effective user ID of the calling process.
 */
PUBLIC pid_t sys_geteuid()
{
	return (curr_proc->euid);
}

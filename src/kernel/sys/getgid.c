/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * getgid.c - getgid() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <sys/types.h>

/*
 * Gets the real group ID of the calling process.
 */
PUBLIC gid_t sys_getgid()
{
	return (curr_proc->gid);
}

/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * getegid.c - getegid() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <sys/types.h>

/*
 *  Gets the effective user group ID of the calling process.
 */
PUBLIC gid_t sys_getegid()
{
	return (curr_proc->egid);
}


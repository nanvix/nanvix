/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/getgid.c> - getgid() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <sys/types.h>

/*
 * Gets the real user group ID of the calling process.
 */
PUBLIC gid_t sys_getgid(void)
{
	return (curr_proc->gid);
}

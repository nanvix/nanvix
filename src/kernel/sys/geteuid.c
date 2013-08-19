/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/geteuid.c> - geteuid() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <sys/types.h>

/*
 *  Gets the effective user ID of the calling process.
 */
PUBLIC uid_t sys_geteuid(void)
{
	return (curr_proc->euid);
}

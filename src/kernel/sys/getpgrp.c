/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/getpgrp.c> - getpgrp() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <sys/types.h>

/*
 * Gets the process group ID of the calling process.
 */
PUBLIC pid_t sys_getpgrp()
{
	return (curr_proc->pgrp->pid);
}

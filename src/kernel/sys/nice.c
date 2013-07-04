/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * nice.c - nice() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <errno.h>
#include <limits.h>

/*
 * Changes the nice value of the calling process.
 */
PUBLIC int sys_nice(int incr)
{
	/* Not authorized. */
	if ((incr < 0) && !IS_SUPERUSER(curr_proc))
		return (-EPERM);
	
	curr_proc->nice += incr;
	
	/* Do not exceed boundaries. */
	if (curr_proc->nice < 0)
		curr_proc->nice = 0;
	else if (curr_proc->nice >= 2*NZERO)
		curr_proc->nice = 2*NZERO - 1;
	
	return (curr_proc->nice);
}


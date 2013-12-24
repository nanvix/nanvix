/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/umask.c - umask() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/pm.h>
#include <sys/types.h>
#include <sys/stat.h>

/*
 * Sets and gets the file mode creation mask.
 */
PUBLIC mode_t sys_umask(mode_t cmask)
{
	mode_t old_mask;
	
	old_mask = curr_proc->umask;
	
	curr_proc->umask = cmask & (MAY_READ | MAY_WRITE | MAY_EXEC);
	
	return (old_mask);
}

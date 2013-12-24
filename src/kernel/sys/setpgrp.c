/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * setpgrp.c - setpgrp() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/pm.h>
#include <sys/types.h>

/*
 * Sets the process group ID.
 */
PUBLIC pid_t sys_setpgrp()
{
	/* Create a new session. */
	if (!IS_LEADER(curr_proc))
	{
		curr_proc->pgrp = curr_proc;
		curr_proc->tty = NULL_DEV;
	}
	
	return (curr_proc->pgrp->pid);
}



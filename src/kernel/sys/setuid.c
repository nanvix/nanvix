/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/setuid.c> - setuid() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <sys/types.h>
#include <errno.h>

/*
 * Sets the real user ID of the calling process.
 */
PUBLIC int sys_setuid(pid_t uid)
{
	/* Superuser authentication. */
	if (IS_SUPERUSER(curr_proc))
	{
		curr_proc->uid = uid;
		curr_proc->euid = uid;
		curr_proc->suid = uid;
	}
	
	else
	{
		/* User authentication. */
		if ((uid == curr_proc->uid) || (uid == curr_proc->suid))
			curr_proc->euid = uid;
		
		/* No authentication. */
		else
			return (-EPERM);
	}
	
	return (0);
}

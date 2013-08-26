/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/setgid.c> - setgid() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <sys/types.h>
#include <errno.h>

/*
 * Sets the user group ID of the calling process.
 */
PUBLIC int sys_setgid(gid_t gid)
{
	/* Superuser authentication. */
	if (IS_SUPERUSER(curr_proc))
	{
		curr_proc->gid = gid;
		curr_proc->egid = gid;
		curr_proc->sgid = gid;
	}
	
	else
	{
		/* User authentication. */
		if ((gid == curr_proc->gid) || (gid == curr_proc->sgid))
			curr_proc->egid = gid;
		
		/* No authentication. */
		else
			return (-EPERM);
	}
	
	return (0);
}

/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/setegid.c> - setegid() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <sys/types.h>
#include <errno.h>

/*
 * Sets the effective user group ID of the calling process.
 */
PUBLIC int sys_setegid(gid_t gid)
{
	/* Superuser authentication. */
	if (IS_SUPERUSER(curr_proc))
		curr_proc->egid = gid;
		
	else
	{
		/* No user authentication. */
		if ((gid != curr_proc->gid) && (gid != curr_proc->sgid))
			return (-EPERM);
		
		curr_proc->egid = gid;
	}
	
	return (0);
}


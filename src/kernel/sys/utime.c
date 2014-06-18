/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/utime.c - utime() system call implementation.
 */

#include <nanvix/clock.h>
#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/mm.h>
#include <utime.h>

/*
 * Internal utime().
 */
PRIVATE void do_utime(struct inode *ip, struct utimbuf *times)
{
	/* Get time. */
	if (times != NULL)
	{
		times->actime = ip->time;
		times->modtime = ip->time;
	}
	
	/* Set time. */
	else
	{
		ip->time = CURRENT_TIME;
		ip->flags |= INODE_DIRTY;
	}
}

/*
 * Set file access and modification times
 */
PUBLIC int sys_utime(const char *path, struct utimbuf *times)
{
	char *name;       /* Inode name.    */
	struct inode *ip; /* Inode pointer. */
	
	/*
	 * Reset errno, since we may
	 * use it in kernel routines.
	 */
	curr_proc->errno = 0;
	
	/* Get time information. */
	if (times != NULL)
	{
		/* Invalid buffer. */
		if (!chkmem(times, sizeof(struct utimbuf), MAY_WRITE))
			goto out0;
	}
	
	name = getname(path);
	
	/* Failed to getname(). */
	if (name == NULL)
		goto out0;
		
	ip = inode_name(name);
	
	/* Failed to get inode. */
	if (ip == NULL)
		goto out1;
	
	do_utime(ip, times);
	
	inode_put(ip);

out1:
	putname(name);
out0:
	return (curr_proc->errno);
}

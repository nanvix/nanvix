/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/stat.c - ustat() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <sys/types.h>
#include <errno.h>
#include <ustat.h>

/*
 * Gets file system statistics.
 */
PUBLIC int sys_ustat(dev_t dev, struct ustat *ubuf)
{
	struct superblock *sb;
	
	/*
	 * Reset errno, since we may
	 * use it in kernel routines.
	 */
	curr_proc->errno = 0;

	/* Valid buffer. */
	if (chkmem(ubuf, sizeof(struct ustat), MAY_WRITE))
		return (-EINVAL);
	
	sb = superblock_get(dev);
	
	/* Not mounted file system. */
	if (sb == NULL)
		return (-ENODEV);
	
	superblock_stat(sb, ubuf);
	
	superblock_put(sb);

	return (curr_proc->errno);
}

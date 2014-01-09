/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/unlink.c - unlink() system call implementation.
 */

#include <nanvix/clock.h>
#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/fs.h>
#include <errno.h>

/*
 * Removes a directory entry.
 */
PUBLIC int sys_unlink(const char *path)
{
	int ret;              /* Return value.      */
	struct inode *dir;    /* Working directory. */
	const char *filename; /* Working file name. */
	char *pathname;       /* Path name.         */
	
	pathname = getname(path);
	
	dir = inode_dname(pathname, &filename);
	
	/* Failed to get directory. */
	if (dir == NULL)
	{
		putname(pathname);
		return (-ENOENT);
	}

	/* No write permissions on directory. */
	if (!permission(dir->mode, dir->uid, dir->gid, curr_proc, MAY_WRITE, 0))
	{
		inode_put(dir);
		putname(pathname);
		return (-EPERM);
	}

	ret = dir_remove(dir, filename);
	
	inode_put(dir);
	putname(pathname);
	
	return (ret);
}

/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/chdir.c - chdir() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/pm.h>
#include <errno.h>

/*
 * Changes working directory.
 */
PUBLIC int sys_chdir(const char *path)
{
	struct inode *inode;
	
	inode = inode_name(path);
	
	/* Failed to get inode. */
	if (inode == NULL)
		return (curr_proc->errno);
	
	/* Not a directory. */
	if (!S_ISDIR(inode->mode))
	{
		inode_put(inode);
		return (-ENOTDIR);
	}
	
	inode_put(curr_proc->pwd);
	
	curr_proc->pwd = inode;
	inode_unlock(inode);
	
	return (0);
}

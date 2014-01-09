/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/access.c - access() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/pm.h>
#include <errno.h>
#include <unistd.h>

/*
 * Checks user permissions for a file.
 */
PUBLIC int sys_access(const char *path, int amode)
{
	mode_t test;      /* Permissions to test.  */
	mode_t mode;      /* Permissions for file. */
	char *name;       /* File name.           */
	struct inode *i;  /* Underlying inode.     */
	
	/* Invalid test. */
	if ((amode & (R_OK | W_OK | X_OK | F_OK)) != amode)
		return (-EINVAL);
	
	name = getname(path);
	
	/* Failed to get name. */
	if (name == NULL)
		return (-ENOENT);
	
	i = inode_name(path);
	
	putname(name);
	
	/* Failed to get inode. */
	if (i == NULL)
		return (curr_proc->errno);
	
	test  = (amode & R_OK) ? MAY_READ : 0;
	test |= (amode & W_OK) ? MAY_WRITE : 0;
	test |= (amode & X_OK) ? MAY_EXEC : 0;
	
	mode = permission(i->mode, i->uid, i->gid, curr_proc, test, 1);
	
	inode_put(i);
	
	return ((test == mode) ? 0 : -EACCES);
}


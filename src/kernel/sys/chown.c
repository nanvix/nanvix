/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/chown.c - chown() system call implementation.
 */

#include <nanvix/clock.h>
#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/pm.h>
#include <sys/stat.h>
#include <errno.h>

/*
 * Changes owner and group of a file.
 */
PUBLIC int sys_chown(const char *path, uid_t owner, gid_t group)
{
	struct inode *inode;
	
	inode = inode_name(path);
	
	/* Failed to get inode. */
	if (inode == NULL)
		return (curr_proc->errno);
	
	/* Not allowed. */
	if ((inode->uid != owner) && (!IS_SUPERUSER(curr_proc)))
	{
		inode_put(inode);
		return (-EACCES);
	}
	
	inode->uid = owner;
	inode->gid = group;
	inode->mode &= ~(S_ISUID | S_ISGID);
	inode_touch(inode);
	inode_put(inode);
	
	return (0);
}

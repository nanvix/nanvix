/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/chmod.c - chmod() system call implementation.
 */

#include <nanvix/clock.h>
#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/pm.h>
#include <sys/stat.h>
#include <errno.h>

/*
 * Changes permissions of a file.
 */
PUBLIC int sys_chmod(const char *path, mode_t mode)
{
	struct inode *inode;
	
	inode = inode_name(path);
	
	/* Failed to get inode. */
	if (inode == NULL)
		return (curr_proc->errno);
	
	/* Not allowed. */
	if ((curr_proc->euid != inode->uid) && (!IS_SUPERUSER(curr_proc)))
	{
		inode_put(inode);
		return (-EACCES);
	}
	
	inode->mode = ((inode->mode & ~(S_ISUID|S_ISGID|S_IRWXU|S_IRWXG|S_IRWXO)) |
	               (mode & (S_ISUID|S_ISGID|S_IRWXU|S_IRWXG|S_IRWXO)));
	    
	/* Clear SGID bit. */
	if ((curr_proc->gid != inode->gid) && !IS_SUPERUSER(curr_proc))
		inode->mode &= ~S_ISGID;
		
	inode->time |= CURRENT_TIME;
	inode->flags |= INODE_DIRTY;
	inode_put(inode);
	
	return (0);
	
}

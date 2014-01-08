/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/stat.h - stat() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/mm.h>
#include <sys/stat.h>
#include <errno.h>

/*
 * Gets file status.
 */
PUBLIC int sys_stat(const char *path, struct stat *buf)
{
	struct inode *inode;
	
	/* Invalid buffer. */
	if (!chkmem(buf, sizeof(struct stat), MAY_WRITE))
		return (-EFAULT);
	
	inode = inode_name(path);
	
	/* Failed to get inode. */
	if (inode == NULL)
		return (curr_proc->errno);
	
	buf->st_dev = inode->dev;
	buf->st_ino = inode->num;
	buf->st_mode = inode->mode;
	buf->st_nlink = inode->nlinks;
	buf->st_uid = inode->uid;
	buf->st_gid = inode->gid;
	buf->st_size = inode->size;
	buf->st_atime = inode->time;
	buf->st_mtime = inode->time;
	buf->st_ctime = inode->time;
	
	return (0);
}

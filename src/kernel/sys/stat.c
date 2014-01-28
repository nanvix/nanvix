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
	struct inode *ip;
	
	/* Invalid buffer. */
	if (!chkmem(buf, sizeof(struct stat), MAY_WRITE))
		return (-EFAULT);
	
	ip = inode_name(path);
	
	/* Failed to get inode. */
	if (ip == NULL)
		return (curr_proc->errno);
	
	buf->st_dev = ip->dev;
	buf->st_ino = ip->num;
	buf->st_mode = ip->mode;
	buf->st_nlink = ip->nlinks;
	buf->st_uid = ip->uid;
	buf->st_gid = ip->gid;
	buf->st_size = ip->size;
	buf->st_atime = ip->time;
	buf->st_mtime = ip->time;
	buf->st_ctime = ip->time;
	
	inode_put(ip);
	
	return (0);
}

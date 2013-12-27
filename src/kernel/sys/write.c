/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/write.c - write() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <errno.h>
#include <nanvix/klib.h>
/*
 * Writes to a file.
 */
PUBLIC ssize_t sys_write(int fd, const void *buf, size_t n)
{
	dev_t dev;       /* Device number.          */
	struct file *f;  /* File.                   */
	struct inode *i; /* Inode.                  */
	ssize_t count;   /* Bytes actually written. */
	
	/* Invalid file descriptor. */
	if ((fd < 0) || (fd > OPEN_MAX) || ((f = curr_proc->ofiles[fd]) == NULL))
		return (-EINVAL);
	
	/* File not opened for writing. */
	if (ACCMODE(f->oflag) == O_RDONLY)
		return (-EBADF);
	
	/* Invalid buffer. */
	if (!chkmem(buf, n, MAY_READ))
		return (-EINVAL);
	
	inode_lock(i = f->inode);
	
	/* Character special file. */
	if (S_ISCHR(i->mode))
	{
		dev = i->zones[0];
		inode_unlock(i);
		count = cdev_write(dev, buf, n);
		return (count);
	}
	
	/* Block special file. */
	else if (S_ISBLK(i->mode))
	{
		dev = i->zones[0];
		inode_unlock(i);
		count = bdev_write(dev, buf, n, f->pos);
		goto out;
	}
	
	/* Pipe file. */
	else if (S_ISFIFO(i->mode))
	{
		kprintf("write to pipe file");
		inode_unlock(i);
		return (-ENOTSUP);
	}
	
	/* Regular file. */
	else if (S_ISREG(i->mode))
		count = file_write(i, buf, n, f->pos);
	
	inode_unlock(i);

out:	

	/* Failed to write. */
	if (count < 0)
		return (curr_proc->errno);
		
	
	f->pos += count;
	
	return (count);
	
}

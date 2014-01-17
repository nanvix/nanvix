/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/write.c - write() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/clock.h>
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
	if ((fd < 0) || (fd >= OPEN_MAX) || ((f = curr_proc->ofiles[fd]) == NULL))
		return (-EBADF);
	
	/* File not opened for writing. */
	if (ACCMODE(f->oflag) == O_RDONLY)
	{
		kprintf("write(%d) %d %x %d", curr_proc->pid, fd, buf, n);
		return (-EBADF);
	}
	
	/* Invalid buffer. */
	if (!chkmem(buf, n, MAY_READ))
		return (-EINVAL);
	
	i = f->inode;
	
	/* Character special file. */
	if (S_ISCHR(i->mode))
	{
		dev = i->zones[0];
		count = cdev_write(dev, buf, n);
		return (count);
	}
	
	/* Block special file. */
	else if (S_ISBLK(i->mode))
	{
		dev = i->zones[0];
		count = bdev_write(dev, buf, n, f->pos);
	}
	
	/* Pipe file. */
	else if (S_ISFIFO(i->mode))
	{
		kprintf("write to pipe");
		count = pipe_write(i, buf, n);
	}
	
	/* Regular file. */
	else if (S_ISREG(i->mode))
		count = file_write(i, buf, n, f->pos);

	/* Failed to write. */
	if (count < 0)
		return (curr_proc->errno);
	
	i->time = CURRENT_TIME;
	f->pos += count;
	
	return (count);
	
}

/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/read.c - read() system call implementation.
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

/*
 * Reads from a file.
 */
PUBLIC ssize_t sys_read(int fd, void *buf, size_t n)
{
	dev_t dev;       /* Device number.       */
	struct file *f;  /* File.                */
	struct inode *i; /* Inode.               */
	ssize_t count;   /* Bytes actually read. */
	
	/* Invalid file descriptor. */
	if ((fd < 0) || (fd >= OPEN_MAX) || ((f = curr_proc->ofiles[fd]) == NULL))
		return (-EBADF);
	
	/* File not opened for reading. */
	if (ACCMODE(f->oflag) == O_WRONLY)
		return (-EBADF);
	
	/* Invalid buffer. */	
	if (!chkmem(buf, n, MAY_WRITE))
		return (-EINVAL);
	
	 i = f->inode;
	
	/* Character special file. */
	if (S_ISCHR(i->mode))
	{		
		dev = i->zones[0];
		count = cdev_read(dev, buf, n);
		return (count);
	}
	
	/* Block special file. */
	else if (S_ISBLK(i->mode))
	{
		dev = i->zones[0];
		count = bdev_read(dev, buf, n, f->pos);
	}
	
	/* Pipe file. */
	else if (S_ISFIFO(i->mode))
	{
		kprintf("read from pipe");
		count = pipe_read(i, buf, n);
	}
	
	/* Regular file/directory. */
	else if ((S_ISDIR(i->mode)) || (S_ISREG(i->mode)))
		count = file_read(i, buf, n, f->pos);
	
	/* Unknown file type. */
	else
		return (-EINVAL);
	
	/* Failed to read. */
	if (count < 0)
		return (curr_proc->errno);

	i->time = CURRENT_TIME;
	f->pos += count;

	return (count);
}

/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/fcntl.c - fcntl() system call implementation.
 */
 
#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/pm.h>
#include <errno.h>
#include <fcntl.h>

/*
 * Duplicates a files descriptor.
 */
PRIVATE int do_dup(int oldfd, int newfd)
{
	/* Invalid file descriptor. */
	if ((newfd < 0) || (newfd >= OPEN_MAX))
		return (-EINVAL);
	
	/* Look for first available file descriptor. */
	while (newfd < OPEN_MAX)
	{
		if (curr_proc->ofiles[newfd] == NULL)
			goto found;
		
		newfd++;
	}

	return (-EMFILE);
	
found:

	curr_proc->close &= ~(1 << newfd);
	(curr_proc->ofiles[newfd] = curr_proc->ofiles[oldfd])->count++;

	return (newfd);
}

/*
 * Manipulates file descriptor.
 */
PUBLIC int sys_fcntl(int fd, int cmd, int arg)
{
	struct file *f;
	
	/* Invalid file descriptor. */
	if ((fd < 0) || (fd >= OPEN_MAX) || ((f = curr_proc->ofiles[fd]) == NULL))
		return (-EBADF);
	
	/* Parse commad. */
	switch (cmd)
	{
		case F_DUPFD :
			return (do_dup(fd, arg));
		
		case F_GETFD :
			return ((curr_proc->close >> fd) & 1);
		
		case F_SETFD :
			if (arg & FD_CLOEXEC)
				curr_proc->close |= 1 << fd;
			else
				curr_proc->close &= ~(1 << fd);
			return (0);
		
		case F_GETFL :
			return (f->oflag);
		
		case F_SETFL :
			f->oflag &= ~(O_APPEND | O_NONBLOCK);
			f->oflag |= arg & (O_APPEND | O_NONBLOCK);
			return (0);
		
		default :
			return (-EINVAL);
	};
}

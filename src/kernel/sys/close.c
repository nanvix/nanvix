/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/close.c - close() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/pm.h>
#include <errno.h>

/*
 * Closes a file.
 */
PUBLIC void do_close(int fd)
{
	struct inode *i; /* Inode. */
	struct file *f;  /* File.  */
	
	f = curr_proc->ofiles[fd];
	
	curr_proc->close &= ~(1 << fd);
	curr_proc->ofiles[fd] = NULL;	
	
	if (--f->count)
		return;
	
	inode_lock(i = f->inode);
	inode_put(i);
}

/*
 * Closes a file.
 */
PUBLIC int sys_close(int fd)
{
	/* Invalid file descriptor. */
	if ((fd < 0) || (fd > OPEN_MAX) || (curr_proc->ofiles[fd] == NULL))
		return (-EINVAL);
	
	do_close(fd);
	
	return (0);
}
 

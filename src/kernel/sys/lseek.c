/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/lseek.c - lseek() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/pm.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <errno.h>
#include <unistd.h>

/*
 * Moves the read/write file offset.
 */
PUBLIC off_t sys_lseek(int fd, off_t offset, int whence)
{
	off_t tmp;      /* Auxiliary variable. */
	struct file *f; /* File.               */
	
	/* Invalid file descriptor. */
	if ((fd < 0) || (fd >= OPEN_MAX) || ((f = curr_proc->ofiles[fd]) == NULL))
		return (-EBADF);
	
	/* Pipe file. */
	if (S_ISFIFO(f->inode->mode))
		return (-ESPIPE);
	
	/* Move read/write file offset. */
	switch (whence)
	{
		case SEEK_CUR :
			/* Invalid offset. */
			if (f->pos + offset < 0)
				return (-EINVAL);
			f->pos += offset;
			break;
		
		case SEEK_END :
			/* Invalid offset. */
			if ((tmp = f->inode->size + offset) < 0)
				return (-EINVAL);
			f->pos = tmp;
			break;
		
		case SEEK_SET :
			/* Invalid offset. */
			if (offset < 0)
				return (-EINVAL);
			f->pos = offset;
			break;
		
		default :
			return (-EINVAL);
			break;
	}
	
	return (f->pos);
}

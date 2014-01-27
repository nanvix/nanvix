/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/ioctl.c - ioctl() system call implementation.
 */

#include <errno.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/fs.h>
#include <nanvix/pm.h>
#include <sys/stat.h>
#include <sys/types.h>

/*
 * Performs control operations on a device.
 */
PUBLIC int sys_ioctl(unsigned fd, unsigned cmd, unsigned arg)
{
	dev_t dev;        /* Underlying device. */
	struct file *fp;  /* Underlying file.   */
	struct inode *ip; /* Underlying inode.  */
	
	/* Invalid file. */
	if ((fd >= OPEN_MAX) || ((fp = curr_proc->ofiles[fd]) == NULL))
		return (-EBADF);
	
	/* Not a character device. */
	if (!S_ISCHR((ip = fp->inode)->mode))
		return (-EINVAL);
	
	dev = ip->zones[0];
	
	return (cdev_ioctl(dev, cmd, arg));
}

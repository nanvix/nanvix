/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * This file is part of Nanvix.
 *
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
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

	dev = ip->blocks[0];

	return (cdev_ioctl(dev, cmd, arg));
}

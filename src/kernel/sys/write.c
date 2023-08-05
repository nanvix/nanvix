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
	dev_t dev;         /* Device number.          */
	struct file *f;    /* File.                   */
	struct inode *i;   /* Inode.                  */
	ssize_t count = 0; /* Bytes actually written. */

	/* Invalid file descriptor. */
	if ((fd < 0) || (fd >= OPEN_MAX) || ((f = curr_proc->ofiles[fd]) == NULL))
		return (-EBADF);

	/* File not opened for writing. */
	if (ACCMODE(f->oflag) == O_RDONLY)
		return (-EBADF);

#if (EDUCATIONAL_KERNEL == 0)

	/* Invalid buffer. */
	if (!chkmem(buf, n, MAY_READ))
		return (-EINVAL);

#endif

	/* Nothing to do. */
	if (n == 0)
		return (0);

	i = f->inode;

	/* Append mode. */
	if (f->oflag & O_APPEND)
		f->pos = i->size;

	/* Character special file. */
	if (S_ISCHR(i->mode))
	{
		dev = i->blocks[0];
		count = cdev_write(dev, buf, n);
		return (count);
	}

	/* Block special file. */
	else if (S_ISBLK(i->mode))
	{
		dev = i->blocks[0];
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

	f->pos += count;

	return (count);
}

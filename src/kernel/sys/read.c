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

#if (EDUCATIONAL_KERNEL == 0)

	/* Invalid buffer. */
	if (!chkmem(buf, n, MAY_WRITE))
		return (-EINVAL);

#endif

	/* Nothing to do. */
	if (n == 0)
		return (0);

	 i = f->inode;

	/* Character special file. */
	if (S_ISCHR(i->mode))
	{
		dev = i->blocks[0];
		count = cdev_read(dev, buf, n);
		return (count);
	}

	/* Block special file. */
	else if (S_ISBLK(i->mode))
	{
		dev = i->blocks[0];
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

	inode_touch(i);
	f->pos += count;

	return (count);
}

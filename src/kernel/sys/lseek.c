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
	}

	return (f->pos);
}

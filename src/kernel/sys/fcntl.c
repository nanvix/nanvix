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
#include <nanvix/syscall.h>
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
 * Duplicates a file descriptor.
 */
EXTERN int sys_dup2(int oldfd, int newfd)
{
	/* Invalid file descriptor. */
	if ((newfd < 0)||(newfd >= OPEN_MAX)||((curr_proc->ofiles[newfd]) == NULL))
		return (-EBADF);

	sys_close(newfd);
	return (do_dup(oldfd, newfd));
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

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
#include <errno.h>

/*
 * Reads data from a pipe.
 */
PUBLIC ssize_t pipe_read(struct inode *inode, char *buf, size_t n)
{
	char *r;

	r = buf;

	/* No writers. */
	wakeup(&inode->chain);
	if (inode->count != 2)
		return (0);

	/* Read from pipe. */
	while (n-- > 0)
	{
		/* Sleep while pipe is empty. */
		while (inode->head == inode->tail)
		{
			/* No writers. */
			wakeup(&inode->chain);
			if (inode->count != 2)
				return (r - buf);

			sleep(&inode->chain, PRIO_INODE);

			/* Awaken by a signal. */
			if (issig())
			{
				curr_proc->errno = -EINTR;
				return (-1);
			}

		}

		*r++ = inode->pipe[inode->tail];
		inode->tail = (inode->tail + 1)%inode->size;
		wakeup(&inode->chain);
	}

	return (r - buf);

}

/*
 * Writes data to a pipe.
 */
PUBLIC ssize_t pipe_write(struct inode *inode, const char *buf, size_t n)
{
	const char *w;

	w = buf;

	/* No readers. */
	wakeup(&inode->chain);
	if (inode->count != 2)
	{
		curr_proc->errno = -EPIPE;
		sndsig(curr_proc, SIGPIPE);
		return (-1);
	}

	/* Write to pipe. */
	while (n-- > 0)
	{
		/* Sleep while pipe is full. */
		while ((inode->head + 1)%inode->size == inode->tail)
		{
			/* No readers. */
			wakeup(&inode->chain);
			if (inode->count != 2)
			{
				curr_proc->errno = -EPIPE;
				sndsig(curr_proc, SIGPIPE);
				return (-1);
			}

			sleep(&inode->chain, PRIO_INODE);

			/* Awaken by a signal. */
			if (issig())
			{
				curr_proc->errno = -EINTR;
				return (-1);
			}
		}

		inode->pipe[inode->head] = *w++;
		inode->head = (inode->head + 1)%inode->size;
		wakeup(&inode->chain);
	}

	return (w - buf);

}

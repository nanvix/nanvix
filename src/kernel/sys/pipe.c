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
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <errno.h>
#include <fcntl.h>

/*
 * Creates an interprocess channel.
 */
PUBLIC int sys_pipe(int fildes[2])
{
	struct file *f[2];   /* Files.            */
	int fd[2];           /* File descriptors. */
	struct inode *inode; /* Pipe inode.       */

	if (!chkmem(fildes, 2*sizeof(int), MAY_WRITE))
		return (-EINVAL);

	/* Get empty file descriptors. */
	if ((fd[0] = getfildes()) < 0)
		return (-EMFILE);
	if ((fd[1] = getfildes()) < 0)
		return (-EMFILE);

	/* Get empty files. */
	if ((f[0] = getfile()) == NULL)
		return (-ENFILE);
	if ((f[1] = getfile()) == NULL)
		return (-ENFILE);

	inode = inode_pipe();

	/* Failed to get pipe inode. */
	if (inode == NULL)
		return (-EAGAIN);

	/* Initialize files. */
	f[0]->oflag = O_RDONLY;
	f[1]->oflag = O_WRONLY;
	f[0]->count = f[1]->count = 1;
	f[0]->inode = f[1]->inode = inode;
	curr_proc->ofiles[fd[0]] = f[0];
	curr_proc->ofiles[fd[1]] = f[1];

	fildes[0] = fd[0];
	fildes[1] = fd[1];

	return (0);
}

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
 * Changes working directory.
 */
PUBLIC int sys_chdir(const char *path)
{
	struct inode *inode;

	inode = inode_name(path);

	/* Failed to get inode. */
	if (inode == NULL)
		return (curr_proc->errno);

	/* Not a directory. */
	if (!S_ISDIR(inode->mode))
	{
		inode_put(inode);
		return (-ENOTDIR);
	}

	inode_put(curr_proc->pwd);

	curr_proc->pwd = inode;
	inode_unlock(inode);

	return (0);
}

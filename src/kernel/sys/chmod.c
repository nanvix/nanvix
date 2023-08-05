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

#include <nanvix/clock.h>
#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/pm.h>
#include <sys/stat.h>
#include <errno.h>

/*
 * Changes permissions of a file.
 */
PUBLIC int sys_chmod(const char *path, mode_t mode)
{
	struct inode *inode;

	inode = inode_name(path);

	/* Failed to get inode. */
	if (inode == NULL)
		return (curr_proc->errno);

	/* Not allowed. */
	if ((curr_proc->euid != inode->uid) && (!IS_SUPERUSER(curr_proc)))
	{
		inode_put(inode);
		return (-EACCES);
	}

	inode->mode = ((inode->mode & ~(S_ISUID|S_ISGID|S_IRWXU|S_IRWXG|S_IRWXO)) |
	               (mode & (S_ISUID|S_ISGID|S_IRWXU|S_IRWXG|S_IRWXO)));

	/* Clear SGID bit. */
	if ((curr_proc->gid != inode->gid) && !IS_SUPERUSER(curr_proc))
		inode->mode &= ~S_ISGID;

	inode_touch(inode);
	inode_put(inode);

	return (0);

}

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
#include <nanvix/klib.h>
#include <nanvix/fs.h>
#include <errno.h>

/*
 * Removes a directory entry.
 */
PUBLIC int sys_unlink(const char *path)
{
	int ret;              /* Return value.      */
	struct inode *dir;    /* Working directory. */
	const char *filename; /* Working file name. */
	char *pathname;       /* Path name.         */

	pathname = getname(path);

	dir = inode_dname(pathname, &filename);

	/* Failed to get directory. */
	if (dir == NULL)
	{
		putname(pathname);
		return (-ENOENT);
	}

	/* No write permissions on directory. */
	if (!permission(dir->mode, dir->uid, dir->gid, curr_proc, MAY_WRITE, 0))
	{
		inode_put(dir);
		putname(pathname);
		return (-EPERM);
	}

	ret = dir_remove(dir, filename);

	inode_put(dir);
	putname(pathname);

	return (ret);
}

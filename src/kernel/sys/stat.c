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
#include <nanvix/mm.h>
#include <sys/stat.h>
#include <errno.h>

/*
 * Gets file status.
 */
PUBLIC int sys_stat(const char *path, struct stat *buf)
{
	struct inode *ip;

	/* Invalid buffer. */
	if (!chkmem(buf, sizeof(struct stat), MAY_WRITE))
		return (-EFAULT);

	ip = inode_name(path);

	/* Failed to get inode. */
	if (ip == NULL)
		return (curr_proc->errno);

	buf->st_dev = ip->dev;
	buf->st_ino = ip->num;
	buf->st_mode = ip->mode;
	buf->st_nlink = ip->nlinks;
	buf->st_uid = ip->uid;
	buf->st_gid = ip->gid;
	buf->st_size = ip->size;
	buf->st_atime = ip->time;
	buf->st_mtime = ip->time;
	buf->st_ctime = ip->time;

	inode_put(ip);

	return (0);
}

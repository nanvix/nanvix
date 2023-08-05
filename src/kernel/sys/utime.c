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
#include <nanvix/mm.h>
#include <utime.h>

/*
 * Internal utime().
 */
PRIVATE void do_utime(struct inode *ip, struct utimbuf *times)
{
	/* Get time. */
	if (times != NULL)
	{
		times->actime = ip->time;
		times->modtime = ip->time;
	}

	/* Set time. */
	else
		inode_touch(ip);
}

/*
 * Set file access and modification times
 */
PUBLIC int sys_utime(const char *path, struct utimbuf *times)
{
	char *name;       /* Inode name.    */
	struct inode *ip; /* Inode pointer. */

	/*
	 * Reset errno, since we may
	 * use it in kernel routines.
	 */
	curr_proc->errno = 0;

	/* Get time information. */
	if (times != NULL)
	{
		/* Invalid buffer. */
		if (!chkmem(times, sizeof(struct utimbuf), MAY_WRITE))
			goto out0;
	}

	name = getname(path);

	/* Failed to getname(). */
	if (name == NULL)
		goto out0;

	ip = inode_name(name);

	/* Failed to get inode. */
	if (ip == NULL)
		goto out1;

	do_utime(ip, times);

	inode_put(ip);

out1:
	putname(name);
out0:
	return (-curr_proc->errno);
}

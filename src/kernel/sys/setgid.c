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
#include <nanvix/pm.h>
#include <sys/types.h>
#include <errno.h>

/*
 * Sets the real user group ID of the calling process.
 */
PUBLIC int sys_setgid(gid_t gid)
{
	/* Superuser authentication. */
	if (IS_SUPERUSER(curr_proc))
	{
		curr_proc->gid = gid;
		curr_proc->egid = gid;
		curr_proc->sgid = gid;
	}

	else
	{
		/* User authentication. */
		if ((gid == curr_proc->gid) || (gid == curr_proc->sgid))
			curr_proc->egid = gid;

		/* No authentication. */
		else
			return (-EPERM);
	}

	return (0);
}

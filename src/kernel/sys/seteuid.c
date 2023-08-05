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
 * Sets the effective user ID of the calling process.
 */
PUBLIC int sys_seteuid(uid_t uid)
{
	/* Superuser authentication. */
	if (IS_SUPERUSER(curr_proc))
		curr_proc->euid = uid;

	else
	{
		/* User authentication. */
		if ((uid == curr_proc->uid) || (uid == curr_proc->suid))
			curr_proc->euid = uid;

		/* No authentication. */
		else
			return (-EPERM);
	}

	return (0);
}

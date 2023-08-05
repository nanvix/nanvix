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
#include <nanvix/dev.h>
#include <nanvix/pm.h>
#include <sys/types.h>

/*
 * Sets the process group ID.
 */
PUBLIC pid_t sys_setpgrp()
{
	/* Create a new session. */
	if (!IS_LEADER(curr_proc))
	{
		curr_proc->pgrp = curr_proc;
		curr_proc->tty = NULL_DEV;
	}

	return (curr_proc->pgrp->pid);
}


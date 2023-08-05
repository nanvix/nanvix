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
#include <errno.h>
#include <signal.h>

/* Sleeping chain. */
PRIVATE struct process *chain = NULL;

/*
 * Suspends the calling process until a signal is received.
 */
PUBLIC int sys_pause()
{
	/* Susped process. */
	while (1)
	{
		sleep(&chain, PRIO_USER);

		/*  Wakeup on signal receipt. */
		if (issig() != SIGNULL)
			goto awaken;
	}

awaken:

	return (-EINTR);
}

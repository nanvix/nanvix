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

#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <nanvix/smp.h>
#include <sys/types.h>
#include <errno.h>


/*
 * @brief Join the specified thread.
 */
PUBLIC int sys_pthread_join(pthread_t thread, void **retval)
{
	struct thread *t;
	struct process *proc;

repeat:
	/* Look for thread to join. */
	for (t = FIRST_THRD; t <= LAST_THRD; t++)
	{
		/* Found. */
		if (t->tid == (tid_t)thread)
		{
			/* Check if joining a peer thread (i.e. in the same proc). */
			if ((proc = thrd_father(t)) == NULL)
				kpanic ("thread scheduled not attached to a process");
			if (proc != curr_proc)
			{
				kprintf ("trying to join a thread from a different process.");
				return (-1);
			}

			/* Join. */
			if (t->state == THRD_TERMINATED)
			{
				clear_thread(t);
				if (t->retval != NULL)
					*retval = t->retval;
				return(0);
			}
			/* Nothing to do, return immediatly. */
			else if (t->state == THRD_DEAD || t->state == THRD_STOPPED)
				return(0);
		}
	}

	/* Wait for a future thread to exit. */
	cpus[curr_core].curr_thread->state = THRD_WAITING;
	yield();

	/* Repeat to check if the exiting thread is the one. */
	goto repeat;

	return (-EINTR);
}

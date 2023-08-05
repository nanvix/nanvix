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
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <sys/types.h>
#include <errno.h>

/* Sleeping chain. */
PRIVATE struct process *chain = NULL;

/*
 * Waits for a child process to terminate.
 */
PUBLIC pid_t sys_wait(int *stat_loc)
{
	int sig;
	pid_t pid;
	struct process *p;

	/* Has no permissions to write at stat_loc. */
	if ((stat_loc != NULL) && (!chkmem(stat_loc, sizeof(int), MAY_WRITE)))
		return (-EINVAL);

repeat:

	/* Nobody to wait for. */
	if (curr_proc->nchildren == 0)
		return (-ECHILD);

	/* Look for child processes. */
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
	{
		/* Skip invalid processes. */
		if (!IS_VALID(p))
			continue;

		 /* Found. */
		if (p->father == curr_proc)
		{
			/* Stopped. */
			if (p->state == PROC_STOPPED)
			{
				/* Already reported. */
				if (p->status)
					continue;

				p->status = 1 << 10;

				/* Get exit code. */
				if (stat_loc != NULL)
					*stat_loc = p->status;

				return (p->pid);
			}

			/* Terminated. */
			else if (p->state == PROC_ZOMBIE)
			{
				/* Get exit code. */
				if (stat_loc != NULL)
					*stat_loc = p->status;

				/*
				 * Get information from child
				 * process before burying it.
				 */
				pid = p->pid;
				curr_proc->cutime += p->utime;
				curr_proc->cktime += p->ktime;

				/* Bury child process. */
				bury(p);

				return (pid);
			}
		}
	}

	sleep(&chain, PRIO_USER);
	sig = issig();

	/* Go back and check what happened. */
	if ((sig == SIGNULL) || (sig == SIGCHLD))
		goto repeat;

	return (-EINTR);
}

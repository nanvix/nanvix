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
#include <signal.h>

/*
 * Asserts if the process p1 is authorized to send a signal to the process p2.
 */
#define AUTHORIZED(p1, p2, s)                          \
	((p2 != IDLE) && (p1 != INIT) &&                   \
	 ((IS_SUPERUSER(p1)) ||                            \
	 ((s == SIGCONT) && (p1->pgrp == p2->pgrp)) ||     \
	 (p1->uid == p2->uid)  || (p1->euid == p2->uid) || \
	 (p1->uid == p2->suid) || (p1->euid == p2->suid))) \

/*
 * Sends a signal to a process or a process group.
 */
PUBLIC int sys_kill(pid_t pid, int sig)
{
	int err;
	struct process *p;

	/* Invalid signal. */
	if ((sig < 0) || (sig > NR_SIGNALS - 1))
		return (-EINVAL);

	err = -ESRCH;

	/* Send signal to process. */
	if (pid > 0)
	{
		/* Search for process. */
		for (p = FIRST_PROC; p <= LAST_PROC; p++)
		{
			/* Skip invalid processes. */
			if (!IS_VALID(p))
				continue;

			/* Found. */
			if (pid == p->pid)
			{
				err = -EPERM;

				if (AUTHORIZED(curr_proc, p, sig))
				{
					err = 0;
					sndsig(p, sig);
				}
			}
		}
	}

	/* Send signal to process group. */
	else if (pid == 0)
	{
		/* Search for processes. */
		for (p = FIRST_PROC; p <= LAST_PROC; p++)
		{
			/* Skip invalid processes. */
			if (!IS_VALID(p))
				continue;

			/* Found. */
			if (curr_proc->pgrp == p->pgrp)
			{
				err = -EPERM;

				if (AUTHORIZED(curr_proc, p, sig))
				{
					err = 0;
					sndsig(p, sig);
				}
			}
		}
	}

	/* Send signal to all processes. */
	else if (pid == -1)
	{
		err = -EPERM;

		/* Search for processes. */
		for (p = FIRST_PROC; p <= LAST_PROC; p++)
		{
			/* Skip invalid processes. */
			if (!IS_VALID(p))
				continue;

			if (AUTHORIZED(curr_proc, p, sig))
			{
				err = 0;
				sndsig(p, sig);
			}
		}
	}

	/* Send signal to absolute proces group. */
	else
	{
		/* Search for processes. */
		for (p = FIRST_PROC; p <= LAST_PROC; p++)
		{
			/* Skip invalid processes. */
			if (!IS_VALID(p))
				continue;

			/* Found. */
			if (p->pgrp->pid == -pid)
			{
				err = -EPERM;

				if (AUTHORIZED(curr_proc, p, sig))
				{
					err = 0;
					sndsig(p, sig);
				}
			}
		}
	}

	return (err);
}

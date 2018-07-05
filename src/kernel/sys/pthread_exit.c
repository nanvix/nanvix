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
#include <nanvix/syscall.h>
#include <nanvix/smp.h>
#include <sys/types.h>
#include <errno.h>

/*
 * @brief Clear a thread from its process.
 */
PUBLIC int clear_thread(struct thread *thrd)
{
	struct thread *tmp_thrd;
	struct process *proc;

	if ((proc = thrd_father(thrd)) == NULL)
		kpanic ("thread scheduled not attached to a process");

	tmp_thrd = proc->threads;
	while (tmp_thrd != NULL)
	{
		/* Remove "head" thread from the list. */
		if (tmp_thrd == thrd)
		{
			proc->threads = tmp_thrd->next;
			goto removed;
		}
		/* Remove "queue" thread from the list. */
		else if (tmp_thrd->next == thrd)
		{
			tmp_thrd->next = thrd->next;
			goto removed;
		}
		tmp_thrd = tmp_thrd->next;
	}

	kprintf("pthread to remove wasn't found in current process");
	return (ESRCH);
removed:
	thrd->state = THRD_DEAD;
	/*
	 * Clear memory.
	 * TODO : should also be able to run cleanup handler.
	 */
	detachreg(proc, &thrd->pregs);
	putkpg(thrd->kstack);

	return (0);
}

/*
 * @brief Terminates a thread.
 */
PUBLIC void sys_pthread_exit(void *retval)
{
	struct thread *tmp_thrd;

	/* Store return value pointer for a future join. */
	cpus[curr_core].curr_thread->retval = retval;

	/*
	 * TODO : if thread is detached, call directly clear_thread without waiting
	 * for this thread to be joined.
	 */
	cpus[curr_core].curr_thread->state = THRD_TERMINATED;
	wakeup_join();

	/* Check if it was the last running thread. */
	tmp_thrd = curr_proc->threads;
	while (tmp_thrd != NULL)
	{
		if (tmp_thrd->state != THRD_TERMINATED && tmp_thrd->state != THRD_DEAD)
			goto remain;
		tmp_thrd = tmp_thrd->next;
	}

	/* If it was the last running thread, exit the process. */
	sys__exit(0);

remain:
	yield();
}

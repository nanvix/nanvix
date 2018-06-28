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
		if (tmp_thrd == thrd)
		{
			/*
			 * Main thread called pthread_exit()
			 * TODO : handle this edge cases : secondary threads should be able
			 * to continue working and the process should terminate when the last
			 * secondary thread terminate.
			 * One secondary thread type flag should be set to THRD_MAIN.
			 */
			kpanic("main thread call pthread_exit");
		}
		/* remove the threads from our linked list */
		else if (tmp_thrd->next == thrd)
		{
			tmp_thrd->next = thrd->next;
			goto removed;
		}
		tmp_thrd = tmp_thrd->next;
	}
	(*(int *)thrd->retval) = -1;
	kpanic("pthread to remove wasn't found in current process");

removed:
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
	/* Store return value pointer for a future join. */
	curr_thread->retval = retval;

	/*
	 * TODO : if thread is detached, call directly clear_thread without waiting
	 * for this thread to be joined.
	 */
	curr_thread->state = THRD_TERMINATED;
	wakeup_join();
	yield();
}

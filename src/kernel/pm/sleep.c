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
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/pm.h>

/**
 * @brief Sleeping chain for idle process.
 */
PRIVATE struct process **idle_chain = NULL;

/**
 * @brief Puts the current process to sleep in a chain of sleeping processes.
 *
 * @details Puts the current process to sleep in the chain of processes pointed
 *          to by @p chain, with a  priority @p priority.
 *
 *          If @p priority if greater than or equal to zero, then the process
 *          is set to an interruptible sleeping state. Otherwise, it is put is
 *          an uninterruptible sleeping state.
 *
 * @param chain    Sleeping chain where the process should be put.
 * @param priority Priority that the process shall assume after waking up.
 */
PUBLIC void sleep(struct process **chain, int priority)
{
	/*
	 * Idle process trying to sleep. Although that may
	 * sound weird, it happens at system startup. So,
	 * let's enable interrupts and do some busy waiting,
	 * hoping that the  interrupt handler will wake us up.
	 */
	if (curr_proc == IDLE)
	{
		idle_chain = chain;
		enable_interrupts();
		while (idle_chain == chain)
			noop();
		return;
	}

	/*
	 * The sleep request is interruptible and the process
	 * has already received a signal, so there is no
	 * need to sleep.
	 */
	if ((priority >= 0) && (curr_proc->received))
		return;

	/* Insert process in the sleeping chain. */
	curr_proc->next = *chain;
	*chain = curr_proc;

	/* Put process to sleep. */
	curr_proc->state = (priority >= 0) ? PROC_WAITING : PROC_SLEEPING;
	curr_proc->priority = priority;
	curr_proc->chain = chain;

	yield();
}

/**
 * @brief Wakes up all processes that are sleeping in a chain.
 *
 * @param chain Chain of sleeping processes to be awaken.
 */
PUBLIC void wakeup(struct process **chain)
{
	/*
	 * Wakeup idle process. Note that here we don't
	 * schedule the idle process for execution, once
	 * we expect that it is the only process in the
	 * system and it is doing some busy-waiting.
	 */
	if (idle_chain == chain)
	{
		idle_chain = NULL;
		return;
	}

	/* Wakeup sleeping processes. */
	while (*chain != NULL)
	{
		sched(*chain);
		*chain = (*chain)->next;
	}
}

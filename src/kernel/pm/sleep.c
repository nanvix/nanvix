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
#include <nanvix/smp.h>

/**
 * @brief Sleeping chain for idle thread.
 */
PRIVATE struct thread **idle_chain = NULL;

/**
 * @brief Puts the current thread to sleep in a chain of sleeping threads.
 * 
 * @details Puts the current thread to sleep in the chain of threads pointed
 *          to by @p chain, with a  priority @p priority.
 * 
 *          If @p priority if greater than or equal to zero, then the thread
 *          is set to an interruptible sleeping state. Otherwise, it is put is
 *          an uninterruptible sleeping state.
 * 
 * @param chain    Sleeping chain where the thread should be put.
 * @param priority Priority that the thread shall assume after waking up.
 */
PUBLIC void sleep(struct thread **chain, int priority)
{
	struct thread *curr_thread;

	/*
	 * Idle thread trying to sleep. Although that may
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
	 * The sleep request is interruptible and the thread
	 * has already received a signal, so there is no
	 * need to sleep.
	 */
	if ((priority >= 0) && (curr_proc->received))
		return;
	
	/* Current thread. */	
	curr_thread = cpus[curr_core].curr_thread;

	/* Insert thread in the sleeping chain. */
	curr_thread->next_thrd = *chain;
	*chain = curr_thread;
	
	/* Put thread to sleep. */
	curr_thread->state = (priority >= 0) ? THRD_WAITING : THRD_SLEEPING;
	curr_thread->priority = priority;
	curr_thread->chain = chain;
	
	yield();
}

/**
 * @brief Wakes up all threads that are sleeping in a chain.
 * 
 * @param chain Chain of sleeping threads to be awaken.
 */
PUBLIC void wakeup(struct thread **chain)
{	
	/*
	 * Wakeup idle thread. Note that here we don't
	 * schedule the idle thread for execution, once
	 * we expect that it is the only thread in the
	 * system and it is doing some busy-waiting. 
	 */
	if (idle_chain == chain)
	{
		idle_chain = NULL;
		return;
	}
	
	/* Wakeup sleeping threads. */
	while (*chain != NULL)
	{
		sched(*chain);
		*chain = (*chain)->next_thrd;
	}
}

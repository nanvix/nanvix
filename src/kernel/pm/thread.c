/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2018 Davidson Francis <davidsondfgl@gmail.com>
 *              2017-2018 Guillaume Besnard <guillaume.besnard@protonmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <nanvix/thread.h>
#include <nanvix/pm.h>
#include <nanvix/klib.h>
#include <nanvix/smp.h>

/**
 * @brief Thread table.
 */
PUBLIC struct thread threadtab[THRD_MAX] = {0};

/**
 * @brief Next available process ID.
 */
PUBLIC tid_t next_tid = 0;

/**
 * @brief Initializes the threads.
 */
PUBLIC void thread_init(void)
{
	/* Current running thread. */
	cpus[CORE_MASTER].curr_thread = THRD_IDLE;
	
	/* Allocate new idle threads for each cpu available. */
	if (smp_enabled)
	{
		struct thread *prev = THRD_IDLE;
		struct thread *curr = THRD_IDLE + 1;
		
		for (int i = 1; i < smp_get_numcores(); i++)
		{
			curr->state = THRD_READY;
			curr->next = NULL;
			curr->flags = 0 << THRD_NEW;
			curr->intlvl = 1;
			curr->kstack = cpus_kstack[i];
			curr->tid = next_tid++;
			curr->counter = PROC_QUANTUM;
			curr->next_thrd = NULL;
			curr->chain = NULL;
			curr->father = IDLE;
			curr->irqlvl = INT_LVL_5;
			curr->pmcs.enable_counters = 0;
			curr->pregs.reg = NULL;
			
			cpus[i].curr_thread = curr;

			prev->next = curr;
			prev = curr;
			curr++;
		}
	}
}

/**
 * @brief Find a free thread.
 */
PUBLIC struct thread *get_free_thread()
{
    struct thread *thrd;

	/* Search for a free thread. */
    for (thrd = FIRST_THRD; thrd <= LAST_THRD; thrd++)
        if (thrd->state == THRD_DEAD) 
			return thrd;

    kprintf("thread table overflow");
    return (NULL);
}

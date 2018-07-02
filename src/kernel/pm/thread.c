/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2017 Davidson Francis <davidsondfgl@hotmail.com>
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

/**
 * @brief Thread table.
 */
PUBLIC struct thread threadtab[THRD_MAX];

/**
 * @brief Current running thread. 
 */
PUBLIC struct thread *curr_thread = THRD_IDLE;

/**
 * @brief Next available process ID.
 */
PUBLIC tid_t next_tid = 0;

/**
 * @brief Find the process owner of a given thread.
 */
PUBLIC struct process *thrd_father(struct thread * thrd)
{
    struct process *p;
    struct thread *tmp_thrd;

	/* Loop through all threads for all processes. */
	for (p = proctab; p <= LAST_PROC; p++)
    {
		tmp_thrd = p->threads;
		while (tmp_thrd != NULL)
		{
			if (tmp_thrd->tid == thrd->tid)
				return p;
			tmp_thrd = tmp_thrd->next;
		}
	}
	kprintf("cannot find father thread");
    return (NULL);
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

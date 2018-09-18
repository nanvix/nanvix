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
 * @brief Marks the thread identified by thread as detached.
 */
PUBLIC int sys_pthread_detach(pthread_t thread)
{
	struct thread *t;

	/* Look for thread to detach. */
    for (t = FIRST_THRD; t <= LAST_THRD; t++)
    {
        /* Found. */
        if (t->tid == (tid_t)thread)
        {
			if (t->detachstate != PTHREAD_CREATE_JOINABLE)
			{
				kprintf("error: detach a not joinable thread.");
				return (ESRCH);
			}
			t->detachstate = PTHREAD_CREATE_DETACHED;
			return (0);
		}
	}

	kprintf("error: detach thread ID wasn't found");
	return (ESRCH);
}

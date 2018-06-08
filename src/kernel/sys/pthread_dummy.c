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
 * Creates a new process.
 */
PUBLIC pid_t sys_pthread_dummy(void)
{
	int err;              /* Error?          */
	struct thread *thrd;  /* Thread.         */
	struct region *reg;   /* Memory region.  */
	struct pregion *preg; /* Process region. */

	kprintf("sys_pthread_dummy");

	/* Search for a free thread */
	for (thrd = FIRST_THRD; thrd <= LAST_THRD; thrd++)
	{
		/* Found. */
		if (thrd->state == THRD_DEAD) {
			thrd->state = THRD_READY;
			thrd->next = NULL;
			thrd->counter = curr_proc->threads->counter;
			thrd->tid = next_tid++;
			curr_proc->threads->next = thrd;
			goto found_thr;
		}
	}

	kprintf("thread table overflow");
	return (-EAGAIN);

found_thr:

	preg = &curr_proc->threads->pregs;

	/* Thread region not in use. */
	if (preg->reg == NULL)
	{
		kpanic("thread region not in use");
		return (-1);
	}

	/* allocate a stack region for the thread */
	if ((reg = allocreg(S_IRUSR | S_IWUSR, PAGE_SIZE, REGION_DOWNWARDS)) == NULL)
		kpanic("allocreg failed");

	/* Failed to duplicate region. */
	if (reg == NULL)
	{
		kpanic("failed to duplicate region");
		return (-1);
	}

	/* Attach the region below previous stack
	 * TODO: different cases, overlap etc */
	err = attachreg(curr_proc, &curr_proc->threads->next->pregs, preg->start - (PGTAB_SIZE), reg);

	/* Failed to attach region. */
	if (err)
	{
		kpanic("failed to attach thread region");
		return (-1);
	}

	/* TODO temporary but force a reload after tlb_flush, useful for debugging purpose */
	kprintf("thread 1 preg %d", &curr_proc->threads->pregs);
	kprintf("thread 1 reg %d", &curr_proc->threads->pregs.reg);
	kprintf("thread 2 preg %d", &curr_proc->threads->next->pregs);
	kprintf("thread 2 reg %d", &curr_proc->threads->next->pregs.reg);

	/* TODO : forge a stack for the thread */

	return (0);
}

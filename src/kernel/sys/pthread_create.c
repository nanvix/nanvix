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
 * Creates a new thread.
 */
PUBLIC int sys_pthread_create(void *pthread, void *attr,
			void *(*start_routine)( void * ), void *arg)
{
	int err;                         /* Error?             */
	struct thread *thrd;             /* Thread.            */
	struct region *reg;              /* Memory region.     */
	struct pregion *preg;            /* Process region.    */
	addr_t start;                    /* Thread stack addr. */

	kprintf("sys_pthread_create");

	if (pthread != NULL || attr != NULL || arg != NULL)
		kpanic("pthread_create arg not null, not supposed to happen for now");

	/* Check start routine address validity. */
	if ((addr_t)start_routine > UBASE_VIRT + REGION_SIZE
				|| (addr_t)start_routine < UBASE_VIRT)
	{
		kpanic("new thread try to access an illegal address");
		return (-1);
	}

	/* Search for a free thread. */
	for (thrd = FIRST_THRD; thrd <= LAST_THRD; thrd++)
	{
		/* Found. */
		if (thrd->state == THRD_DEAD) {
			thrd->state = THRD_READY;
			thrd->next = NULL;
			thrd->counter = curr_proc->threads->counter;
			thrd->tid = next_tid++;
			thrd->flags = THRD_NEW;
			thrd->flags = 1 << THRD_NEW;
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

	/* Allocate a stack region for the thread. */
	if ((reg = allocreg(S_IRUSR | S_IWUSR, PAGE_SIZE, REGION_DOWNWARDS)) == NULL)
		kpanic("allocreg failed");

	/* Failed to duplicate region. */
	if (reg == NULL)
	{
		kpanic("failed to duplicate region");
		return (-1);
	}

	/* Attach the region below previous stack. */
	start = preg->start - PGTAB_SIZE;
	err = attachreg(curr_proc, &curr_proc->threads->next->pregs, start, reg);

	/* Failed to attach region. */
	if (err)
	{
		kpanic("failed to attach thread region");
		return (-1);
	}
	unlockreg(reg);

	/* TODO temporary but force a reload after tlb_flush, useful for debugging purpose */
	kprintf("thread 1 preg %d", &curr_proc->threads->pregs);
	kprintf("thread 1 reg %d", &curr_proc->threads->pregs.reg);
	kprintf("thread 2 preg %d", &curr_proc->threads->next->pregs);
	kprintf("thread 2 reg %d", &curr_proc->threads->next->pregs.reg);

#if or1k
	addr_t sp = (start - (INT_FRAME_SIZE)) & ~(DWORD_SIZE - 1); /* align kesp. */

	/* Forge a stack for the thread. */
	forge_stack(sp, start_routine);

	curr_proc->threads->next->kstack = (void *)start;
	curr_proc->threads->next->kesp = sp;

	kprintf("sp %d", sp);
	kprintf("routine %d", (addr_t)start_routine);
	kprintf("stack thread read sp %d", (*((dword_t *)(curr_proc->threads->next->kesp + SP))));
	kprintf("stack thread read EPCR %d", (*((dword_t *)(curr_proc->threads->next->kesp + EPCR))));
#endif

	sched_thread(curr_proc, curr_proc->threads->next);
	return (0);
}

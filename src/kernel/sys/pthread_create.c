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
 * @brief Forge a "fake" stack for the new threads
 * and adjust the kernel stack pointer accordingly.
 */
PRIVATE int setup_stack(addr_t user_sp, void *(*start_routine)( void * ))
{
	void *kstack;   /* Kernel stack underlying page. */
	addr_t kern_sp; /* Kernel stack pointer.         */

	/* Get kernel page for kernel stack. */
    kstack = getkpg(0);
	if (kstack == NULL)
	{
		kprintf("cannot allocate kstack");
		goto error;
	}

	/* Forge the new stack and update the thread structure */
	kern_sp = forge_stack(kstack, start_routine, user_sp);
	curr_thread->next->kstack = kstack;
	curr_thread->next->kesp = kern_sp;
	return (0);

error:
	return (-1);
}

/*
 * @brief Allocate and attach new stack region for the new thread.
 */
PRIVATE addr_t alloc_attach_stack_reg()
{
	int err;              /* Error?                 */
	struct pregion *preg; /* Process region.        */
	struct region *reg;   /* Memory region.         */
	addr_t start;         /* New region start addr. */

	preg = &curr_proc->threads->pregs;

	/* Thread region not in use. */
	if (preg->reg == NULL)
	{
		kprintf("thread region not in use");
		return (-1);
	}

	/* Allocate a stack region for the thread. */
	if ((reg = allocreg(S_IRUSR | S_IWUSR, PAGE_SIZE, REGION_DOWNWARDS)) == NULL)
		kpanic("allocreg failed");

	/* Failed to duplicate region. */
	if (reg == NULL)
	{
		kprintf("failed to duplicate region");
        goto error;
	}

	/* Attach the region below previous stack. */
	start = (preg->start - PGTAB_SIZE);
	err = attachreg(curr_proc, &curr_thread->next->pregs, start, reg);

	/* Failed to attach region. */
	if (err)
	{
		kpanic("failed to attach thread region");
		freereg(reg);
		goto error;
	}
	unlockreg(reg);
	return (start - DWORD_SIZE);

error:
	return (0);
}

/*
 * @brief Creates a new thread.
 */
PUBLIC int sys_pthread_create(void *pthread, void *attr,
							  void *(*start_routine)( void * ),
							  void *arg)
{
	struct thread *thrd;             /* Thread.            */
	addr_t user_sp;                  /* User stack addr.   */

	kprintf("sys_pthread_create");

	if (pthread != NULL || attr != NULL || arg != NULL)
		kpanic("pthread_create arg not null, not supposed to happen for now");

	/* Check start routine address validity. */
	if ((addr_t)start_routine > UBASE_VIRT + REGION_SIZE
				|| (addr_t)start_routine < UBASE_VIRT)
	{
		kpanic("new thread try to access an illegal address");
		goto error;
	}

	/* Find and init a new thread structure. */
	if ((thrd = get_free_thread()) == NULL)
		goto error;

	thrd->state = THRD_READY;
	thrd->next = NULL;
	thrd->counter = curr_proc->threads->counter;
	thrd->tid = next_tid++;
	thrd->flags = THRD_NEW;
	thrd->flags = 1 << THRD_NEW;
	curr_proc->threads->next = thrd;

	/* Allocate and setup the new stack. */
	if ((user_sp = alloc_attach_stack_reg()) == 0)
		goto error;

	if (setup_stack(user_sp, start_routine) == -1)
		goto error;

	/* Schedule our new thread to run. */
	sched_thread(curr_proc, curr_proc->threads->next);
	return (0);

error:
	kprintf("pthread_create error.");
	return (-1);
}

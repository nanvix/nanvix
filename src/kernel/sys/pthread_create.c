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
PRIVATE int setup_stack(addr_t user_sp, void *arg,
						struct thread *thrd,
						void *(*start_routine)( void * ),
						void (*start_thread)( void * ))
{
	void *kstack;    /* Kernel stack underlying page. */
	void *ipikstack; /* IPI kernel stack.             */
	addr_t kern_sp;  /* Kernel stack pointer.         */

	/* Get kernel page for kernel stack. */
	kstack = getkpg(0);
	if (kstack == NULL)
	{
		kprintf("cannot allocate kstack");
		goto error0;
	}

	/* Get kernel page for IPI kernel stack. */
	ipikstack = getkpg(1);
	if (ipikstack == NULL)
	{
		kprintf("cannot allocate ipi kstack");
		goto error1;
	}	

	/* Forge the new stack and update the thread structure. */
	kern_sp = forge_stack(kstack, start_routine, user_sp, arg, start_thread);
	thrd->kstack = kstack;
	thrd->kesp = kern_sp;
	thrd->ipikstack = ipikstack;

	return (0);

error1:
	putkpg(kstack);
error0:
	return (-1);
}

/*
 * @brief Allocate and attach new stack region for the new thread.
 */
PRIVATE addr_t alloc_attach_stack_reg(struct thread *thrd)
{
	int err;              /* Error?                 */
	struct pregion *preg; /* Process region.        */
	struct region *reg;   /* Memory region.         */
	addr_t start;         /* New region start addr. */
	addr_t offset;        /* Reg addr offfset.      */

	/* Allocate a stack region for the thread. */
	if ((reg = allocreg(S_IRUSR | S_IWUSR, PAGE_SIZE, REGION_DOWNWARDS)) == NULL)
		kpanic("allocreg failed");

	/* Failed to duplicate region. */
	if (reg == NULL)
	{
		kprintf("failed to duplicate region");
        goto error;
	}


	preg = &curr_proc->threads->pregs;

	/* Thread region not in use. */
	if (preg->reg == NULL)
	{
		kprintf("thread region not in use");
		freereg(reg);
		goto error;
	}

	/* Get a correct offset to attach new stack. */
	offset = 0;
	do {
		offset += PGTAB_SIZE;
		start = (preg->start - offset);
	} while (!addr_is_clear(curr_proc, start));

	/* Attach the region below previous stack. */
	err = attachreg(curr_proc, &thrd->pregs, start, reg);

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
PUBLIC int sys_pthread_create(pthread_t *pthread, _CONST pthread_attr_t *attr,
							  void *(*start_routine)( void * ), void *arg,
							  void (*start_thread)( void * ))
{
	struct thread *thrd;             /* Thread.            */
	struct thread *t;                /* Tmp thread.        */
	addr_t user_sp;                  /* User stack addr.   */


	/* Check start routine address validity. */
	if (start_routine == NULL
			|| (addr_t)start_routine < UBASE_VIRT
			|| (addr_t)start_routine > UBASE_VIRT + REGION_SIZE)
	{
		kprintf("pthread_create : start_routine illegal address");
		goto error;
	}

	/* Find and init a new thread structure. */
	if ((thrd = get_free_thread()) == NULL)
		goto error;

	thrd->state = THRD_READY;
	thrd->next = NULL;
	thrd->counter = curr_proc->threads->counter;
	thrd->intlvl = 1;
	thrd->tid = next_tid++;
	thrd->flags = 1 << THRD_NEW;
	thrd->retval = NULL;
	thrd->father = curr_proc;
	*pthread = thrd->tid;

	/*
	 * Check pthread attributes.
	 * TODO: For now, we only handle detachstate attribute.
	 */
	if (attr != NULL)
		thrd->detachstate = attr->detachstate;
	else
	    thrd->detachstate = PTHREAD_CREATE_JOINABLE;

	/* Attach new thread in the process list. */
	t = curr_proc->threads;
	while(t->next != NULL)
		t = t->next;
	t->next = thrd;

	/* Allocate and setup the new stack. */
	if ((user_sp = alloc_attach_stack_reg(thrd)) == 0)
		goto error;

	if (setup_stack(user_sp, arg, thrd, start_routine, start_thread) == -1)
		goto error;

	/* Schedule our new thread to run. */
	sched(thrd);
	return (0);

error:
	kprintf("pthread_create error.");
	return (-1);
}

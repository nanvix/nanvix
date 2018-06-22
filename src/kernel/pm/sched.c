/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2017 Davidson Francis <davidsondfgl@hotmail.com>
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

#include <nanvix/clock.h>
#include <nanvix/const.h>
#include <nanvix/hal.h>
#include <nanvix/pm.h>
#include <signal.h>

/**
 * @brief Schedules a process to execution.
 * 
 * @param proc Process to be scheduled.
 */
PUBLIC void sched(struct process *proc)
{
	struct thread *t;

	t = proc->threads;
	while (t != NULL)
	{
		t->state = THRD_READY;
		t->counter = 0;
		t = t->next;
	}
	proc->state = PROC_READY;
}

/**
 * @brief Schedules only a thread to execution.
 *
 * @param proc Process to be scheduled.
 * @param thrd Thread to be scheduled.
 */
PUBLIC void sched_thread(struct process *proc, struct thread *thrd)
{
	struct thread *t;

	proc->state = PROC_READY;
	thrd->state = THRD_READY;
	thrd->counter = 0;

	/* test if there was already a thread running in this process */
	t = proc->threads;
	while (t != NULL)
	{
		if (t->state == THRD_RUNNING)
			proc->state = PROC_RUNNING;
		t = t->next;
	}
}


/**
 * @brief Stops the current running process.
 */
PUBLIC void stop(void)
{
	struct thread *t;
	curr_proc->state = PROC_STOPPED;

	while (t != NULL)
	{
		t->state = THRD_STOPPED;
		t = t->next;
	}

	sndsig(curr_proc->father, SIGCHLD);
	yield();
}

/**
 * @brief Resumes a process.
 * 
 * @param proc Process to be resumed.
 * 
 * @note The process must stopped to be resumed.
 */
PUBLIC void resume(struct process *proc)
{	
	/* Resume only if process has stopped. */
	if (proc->state == PROC_STOPPED)
		sched(proc);
}

/**
 * @brief Yields the processor.
 */
PUBLIC void yield(void)
{
	struct process *p;        /* Working process.     */
	struct process *next;     /* Next process to run. */
	struct thread *t;         /* Working thread.      */
	struct thread *next_thrd; /* Next thread  to run. */

	/* Re-schedule process for execution. */
	if (curr_proc->state == PROC_RUNNING)
	{
		sched_thread(curr_proc, curr_thread);

		/* Checks if the current process have an active counter. */
		if (curr_thread->pmcs.enable_counters != 0)
		{
			/* Save the current counter. */
			if (curr_thread->pmcs.enable_counters & 1)
				curr_thread->pmcs.C1 += read_pmc(0);
			
			if (curr_thread->pmcs.enable_counters >> 1)
				curr_thread->pmcs.C2 += read_pmc(1);
			/* Reset the counter. */
			pmc_init();
		}
	}

	/* Remember this process. */
	last_proc = curr_proc;

	/* Check alarm. */
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
	{
		/* Skip invalid processes. */
		if (!IS_VALID(p))
			continue;
		
		/* Alarm has expired. */
		if ((p->alarm) && (p->alarm < ticks))
			p->alarm = 0, sndsig(p, SIGALRM);
	}

	/* Choose a thread to run next. */
	next_thrd = IDLE->threads;
	next = IDLE;

	for (t = FIRST_THRD; t <= LAST_THRD; t++)
	{
		/* Skip non-ready thread. */
		if (t->state != THRD_READY)
			continue;

		/*
		 * Thread with higher
		 * waiting time found.
		 */
		if (t->counter > next_thrd->counter)
		{
			next_thrd->counter++;
			next_thrd = t;
			if ((next = thrd_father(next_thrd)) == NULL)
				kpanic ("thread scheduled not attached to a process");
		}

		/*
		 * Increment waiting
		 * time of thread.
		 */
		else
			t->counter++;
	}

	if (next_thrd->state != THRD_READY)
		kpanic("thread elected incoherent state : not ready");

	/* Switch to next process. */
	next->priority = PRIO_USER;
	next->state = PROC_RUNNING;
	next_thrd->state = THRD_RUNNING;
	next_thrd->counter = PROC_QUANTUM;

	/* Start performance counters. */
	if (next_thrd->pmcs.enable_counters != 0)
	{
		/* Enable counters. */
		write_msr(IA32_PERF_GLOBAL_CTRL, IA32_PMC0 | IA32_PMC1);

		/* Starts the counter 1. */
		if (next_thrd->pmcs.enable_counters & 1)
		{
			uint64_t value = IA32_PERFEVTSELx_EN | IA32_PERFEVTSELx_USR
				| next_thrd->pmcs.event_C1;

			write_msr(IA32_PERFEVTSELx, value);
		}
		
		/* Starts the counter 2. */
		if (next_thrd->pmcs.enable_counters >> 1)
		{
			uint64_t value = IA32_PERFEVTSELx_EN | IA32_PERFEVTSELx_USR
				| next_thrd->pmcs.event_C2;

			write_msr(IA32_PERFEVTSELx + 1, value);
		}
	}
	switch_to(next, next_thrd);
}

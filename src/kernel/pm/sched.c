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
 * @brief Calculates process' real priority.
 *
 * @details Calculates process' effective priority by
 * considering static, dynamic and nice priorities. The
 * lower this value is, higher is the process' priority.
 *
 * @param p Process that will have its priority calculated.
 *
 * @returns Integer value (negative or positive) that tells
 * what is the effective priority of @p p.
 */
#define PRIORITY(p) ((p)->priority + (p)->nice - (p)->counter)


/**
 * @brief Schedules a process to execution.
 * 
 * @param proc Process to be scheduled.
 */
PUBLIC void sched(struct process *proc)
{
	proc->state = PROC_READY;
	proc->counter = 0;
}

/**
 * @brief Stops the current running process.
 */
PUBLIC void stop(void)
{
	curr_proc->state = PROC_STOPPED;
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
	struct process *p;    /* Working process.     */
	struct process *next; /* Next process to run. */

	/* Re-schedule process for execution. */
	if (curr_proc->state == PROC_RUNNING)
	{
		sched(curr_proc);

		/* Checks if the current process have an active counter. */
		if (curr_proc->pmcs.enable_counters != 0)
		{
			/* Save the current counter. */
			if (curr_proc->pmcs.enable_counters & 1)
				curr_proc->pmcs.C1 += read_pmc(0);
			
			if (curr_proc->pmcs.enable_counters >> 1)
				curr_proc->pmcs.C2 += read_pmc(1);

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

	/* Choose a process to run next. */
	next = IDLE;
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
	{
		/* Skip non-ready process. */
		if (p->state != PROC_READY)
			continue;
		
		int p_real_priority = PRIORITY(p);
		int next_real_priority = PRIORITY(next);

		/*
		 * The next chosen process
		 * should be one of those with the
		 * highest priority found which has been
		 * waiting for the longest time.
		 */
		if (p_real_priority < next_real_priority ||
			(p_real_priority == next_real_priority &&
			 p->counter > next->counter))
		{
			next->counter++;
			next = p;
		}
 
		/*
		 * Increment waiting
		 * time of process.
		 */
		else
			p->counter++;
	}
	
	/* Switch to next process. */
	next->priority = PRIO_USER;
	next->state = PROC_RUNNING;
	next->counter = PROC_QUANTUM;

	/* Start performance counters. */
	if (next->pmcs.enable_counters != 0)
	{
		/* Enable counters. */
		write_msr(IA32_PERF_GLOBAL_CTRL, IA32_PMC0 | IA32_PMC1);

		/* Starts the counter 1. */
		if (next->pmcs.enable_counters & 1)
		{
			uint64_t value = IA32_PERFEVTSELx_EN | IA32_PERFEVTSELx_USR
				| next->pmcs.event_C1;

			write_msr(IA32_PERFEVTSELx, value);
		}
		
		/* Starts the counter 2. */
		if (next->pmcs.enable_counters >> 1)
		{
			uint64_t value = IA32_PERFEVTSELx_EN | IA32_PERFEVTSELx_USR
				| next->pmcs.event_C2;

			write_msr(IA32_PERFEVTSELx + 1, value);
		}
	}
	
	switch_to(next);
}

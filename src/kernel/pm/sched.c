/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2018 Davidson Francis <davidsondfgl@hotmail.com>
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
#include <nanvix/klib.h>
#include <nanvix/smp.h>
#include <or1k/ompic.h>
#include <signal.h>

/**
 * Yields the processor.
 */
PUBLIC void (*yield)(void);

/**
 * @brief Schedules a thread to execution.
 * 
 * @param thrd Thread to be scheduled.
 */
PUBLIC void sched(struct thread *thrd)
{
	thrd->state = THRD_READY;
	thrd->counter = 0;
}

/**
 * @brief Schedules a process to execution.
 *
 * @param proc Process to be scheduled.
 */
PUBLIC void sched_process(struct process *proc)
{
	struct thread *t;
	proc->state = PROC_READY;
	t = proc->threads;
	
	while (t != NULL)
	{
		if (t->state == THRD_RUNNING)
		{
			t->state = THRD_READY;
			t->counter = 0;
		}
		t = t->next;
	}
}

/**
 * @brief Schedules a blocking thread for the current process.
 *
 * @param next Reference process to be queried.
 */
PUBLIC void sched_blocking_thread(struct process *next)
{
	struct thread *next_thrd; /* Next thread to run. */
	unsigned i;               /* Loop index.         */

	next_thrd = next->threads;
	i = 1;

	while (next_thrd != NULL)
	{
		if (next_thrd->state != THRD_READY || !(next_thrd->flags & THRD_SYS))
		{
			next_thrd = next_thrd->next;
			i++;
			continue;
		}

		if (cpus[i].state != CORE_READY)
			kpanic("yield_smp: core %d not ready yet!", i);

		next_thrd->state = THRD_RUNNING;
		next_thrd->ipi.exception_handler = 0;
		
		cpus[i].curr_proc = next;
		cpus[i].curr_thread = next_thrd;
		cpus[i].next_thread = next_thrd;
		
		curr_core = i;
		curr_proc = next;
		ompic_send_ipi(i, IPI_SCHEDULE);
		switch_to(next, next_thrd);
	}
}

/**
 * @brief Wakeup all potential joining thread.
 */
PUBLIC void wakeup_join()
{
	struct thread *t;
	t = curr_proc->threads;
	while (t != NULL)
	{
		if (t->state == THRD_WAITING)
			sched(t);
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
		sched(proc->threads);
}

/**
 * @brief Checks if a given process is ready to run.
 *
 * @param proc Process to be checked.
 *
 * @return Returns 1 if a given process have at least
 * one thread that is ready, 0 otherwise.
 */
PUBLIC int process_is_ready(struct process *proc)
{
	struct thread *t;
	t = proc->threads;

	while (t != NULL)
	{
		if (t->state == THRD_READY)
			return (1);

		t = t->next;
	}

	return (0);
}

/**
 * @brief Yields the processor while in UP.
 */
PUBLIC void yield_up(void)
{
	struct process *p;        /* Working process.     */
	struct process *next;     /* Next process to run. */
	struct thread *t;         /* Working thread.      */
	struct thread *next_thrd; /* Next thread  to run. */

	/* Re-schedule process for execution. */
	if (curr_proc->state == PROC_RUNNING
			&& cpus[CORE_MASTER].curr_thread->state == THRD_RUNNING)
	{
		sched(cpus[CORE_MASTER].curr_thread);

		/* Checks if the current process have an active counter. */
		if (cpus[curr_core].curr_thread->pmcs.enable_counters != 0)
		{
			/* Save the current counter. */
			if (cpus[curr_core].curr_thread->pmcs.enable_counters & 1)
				cpus[curr_core].curr_thread->pmcs.C1 += read_pmc(0);
			
			if (cpus[curr_core].curr_thread->pmcs.enable_counters >> 1)
				cpus[curr_core].curr_thread->pmcs.C2 += read_pmc(1);

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
			if ((next = next_thrd->father) == NULL)
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
	
	/* Switch proceses. */
	curr_proc = next;
	switch_to(next, next_thrd);
}

/**
 * @brief Yields the processor while in SMP.
 */
PUBLIC void yield_smp(void)
{
	struct process *p;        /* Working process.     */
	struct process *next;     /* Next process to run. */
	struct thread *next_thrd; /* Next thread  to run. */
	unsigned i;               /* Loop index.          */

	/* Slave cores are not allowed here. */
	if (smp_get_coreid() != CORE_MASTER)
		kpanic("yield_smp: slave cores cannot yield");

	/* If serving a slave core, saves the context. */
	if (curr_core != CORE_MASTER && cpus[curr_core].curr_thread->flags
		& (1 << THRD_SYS))
	{
		save_ipi_context();
		return;
	}

	/**
	 * Checks if there is at least one thread that is waiting for an IPI.
	 */
	serving_ipis = 1;
	next_thrd = curr_proc->threads;
	while (next_thrd != NULL)
	{
		if (next_thrd->ipi.waiting_ipi && next_thrd->state != THRD_WAITING)
			ompic_handle_ipi();

		next_thrd = next_thrd->next;
	}
	serving_ipis = 0;

	/**
	 * Ask and waits for other cores to stop current thread.
	 * It's important to note that the usage of spinlocks below is
	 * because we need to ensure that all the slave cores are
	 * available to run a new process before we start to sending
	 * IPIs to the cores to run the threads.
	 */
	for (i = 1; i < smp_get_numcores(); i++)
	{
		if (cpus[i].state == CORE_RUNNING)
		{
			spin_lock(&ipi_lock);
			ompic_send_ipi(i, IPI_IDLE);
		}
	}
	spin_lock(&ipi_lock);
	spin_unlock(&ipi_lock);

	/* Re-schedule process for execution. */
	if (curr_proc->state == PROC_RUNNING)
		sched_process(curr_proc);
	
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
	next = INIT;
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
	{
		/* Skip non-ready process. */
		if (!process_is_ready(p))
			continue;
		
		/*
		 * Process with higher
		 * waiting time found.
		 */
		if (p->counter >= next->counter)
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
	next->state = PROC_RUNNING;
	next->counter = PROC_QUANTUM;

	/* Send an IPI for each remaining core. */
	next_thrd = next->threads;
	i = 1;

	/* Switch processes. */
	curr_proc = next;
	
	/* Re-schedule non-blocking threads. */
	while (next_thrd != NULL)
	{
		if (next_thrd->state != THRD_READY || next_thrd->flags & THRD_SYS)
		{
			next_thrd = next_thrd->next;
			i++;
			continue;
		}

		if (cpus[i].state != CORE_READY)
			kpanic("yield_smp: core %d not ready yet!", i);

		next_thrd->state = THRD_RUNNING;
		next_thrd->ipi.exception_handler = 0;

		cpus[i].curr_proc = next;
		cpus[i].next_thread = next_thrd;
		ompic_send_ipi(i, IPI_SCHEDULE);
		
		next_thrd = next_thrd->next;
		i++;
	}

	/* Re-schedule blocking threads, if exist. */
	sched_blocking_thread(next);
}

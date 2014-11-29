/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sched.c - Process scheduler.
 */

#include <nanvix/clock.h>
#include <nanvix/const.h>
#include <nanvix/hal.h>
#include <nanvix/pm.h>
#include <signal.h>

/*
 * Returns the effective process priority.
 */
#define EPRIO(p)                         \
	(p->priority + p->nice - p->counter) \

/*
 * Stops the current running process.
 */
PUBLIC void stop(void)
{
	curr_proc->state = PROC_STOPPED;
	
	yield();
}

/*
 * Resumes a process.
 */
PUBLIC void resume(struct process *proc)
{	
	/* Resume only if process has stopped. */
	if (proc->state == PROC_STOPPED)
		sched(proc);
}

/*
 * Schedules a process to execution.
 */
PUBLIC void sched(struct process *proc)
{
	proc->state = PROC_READY;
	proc->counter = 0;
}

/*
 * Yields the processor.
 */
PUBLIC void yield(void)
{
	int eprio;
	struct process *p;
	struct process *next;

	/* Re-schedule process for execution. */
	if (curr_proc->state == PROC_RUNNING)
		sched(curr_proc);

	/* Check alarm. */
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
	{
		/* Valid process. */
		if (p->state != PROC_DEAD)
		{
			/* Alarm has expired. */
			if ((p->alarm) && (p->alarm < ticks))
				p->alarm = 0, sndsig(p, SIGALRM);
		}
	}

	/* Choose a process to execute. */
	next = IDLE;
	eprio = EPRIO(next);
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
	{
		/* Skip invalid processes. */
		if (p->flags & PROC_FREE)
			continue;
		
		/* Skip non-ready process. */
		if (p->state != PROC_READY)
			continue;
		
		/* Process with higher priority found. */
		if (EPRIO(p) < eprio)
		{
			next->counter++;
			next = p;
			eprio = EPRIO(next);
		}
			
		/* Increment age of process. */
		else
			p->counter++;
	}
	
	/* Switch to process. */
	next->priority = PRIO_USER;
	next->state = PROC_RUNNING;
	next->counter = PROC_QUANTUM;
	switch_to(next);
}

/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * die.c - Exceptions.
 */

#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/pm.h>
#include <signal.h>

/*
 * 
 */
PUBLIC void die(int code)
{
	int i;
	struct process *p;
	struct region *reg;
	
	/* Shall not occour. */
	if (curr_proc == INIT)
		kpanic("init died");
	
	/* Ignore all signals since, process may sleep below. */
	for (i = 0; i < NR_SIGNALS; i++)
		curr_proc->handlers[i] = SIG_IGN;
	
	/* init adopts child processes. */
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
	{
		/* Child process found. */
		if (p->father == curr_proc)
			p->father == INIT;
	}
	
	/* init adotps process in the same group. */
	if (curr_proc->pgrp == curr_proc->pid)
	{
		for (p = FIRST_PROC; p <= LAST_PROC; p++)
		{
			if (p->pgrp == curr_proc->pid)
			{
				sndsig(p, SIGHUP);
				sndsig(p, SIGCONT);
			}
		}
	}
	
	/* Detach attached memory regions. */
	for (i = 0; i < NR_PREGIONS; i++)
	{
		/* Attached region found. */
		if (curr_proc->pregs[i].type != PREGION_UNUSED)
		{
			/* Detach. */
			lockreg(curr_proc->pregs[i].reg);
			reg = detachreg(curr_proc, &curr_proc->pregs[i]);
			freereg(reg);
		}
	}

	curr_proc->state = PROC_ZOMBIE;
	curr_proc->status = status;
	
	sndsig(curr_proc->father, SIGCHLD);
	
	yield();
}

/*
 * Terminates a process.
 */
PUBLIC void terminate(int err)
{
	kprintf("terminating process %d with error code %d", curr_proc->pid, err);
	
	die();
}

/*
 * Aborts the execution of a process.
 */
PUBLIC void abort()
{
	kprintf("aborting process %d", curr_proc->pid);
	
	die();
}

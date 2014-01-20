/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * die.c - Exceptions.
 */

#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/pm.h>
#include <nanvix/paging.h>
#include <signal.h>

/*
 * 
 */
PUBLIC void die(int status)
{
	int i;
	struct process *p;
	
	/* Shall not occour. */
	if (curr_proc == IDLE)
		kpanic("idle process dying");
	
	curr_proc->status = status;
	
	/* Ignore all signals since, process may sleep below. */
	for (i = 0; i < NR_SIGNALS; i++)
		curr_proc->handlers[i] = SIG_IGN;
	
	/* init adopts orphan processes. */
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
	{
		/* Child process found. */
		if (p->father == curr_proc)
			p->father = INIT;
	}
	
	/* init adotps process in the same group. */
	if (curr_proc->pgrp == curr_proc)
	{
		for (p = FIRST_PROC; p <= LAST_PROC; p++)
		{
			if (p->pgrp == curr_proc)
			{
				p->pgrp = NULL;
				sndsig(p, SIGHUP);
				sndsig(p, SIGCONT);
			}
		}
	}
	
	/* Detach process memory regions. */
	for (i = 0; i < NR_PREGIONS; i++)
		detachreg(curr_proc, &curr_proc->pregs[i]);

	curr_proc->state = PROC_ZOMBIE;
	curr_proc->alarm = 0;
	
	sndsig(curr_proc->father, SIGCHLD);
	
	yield();
}

/*
 * Terminates a process.
 */
PUBLIC void terminate(int sig)
{
	die(((sig & 0xff) << 16) | (1 << 9));
}

/*
 * Aborts the execution of a process.
 */
PUBLIC void abort(int sig)
{
	die(((sig & 0xff) << 16) | (1 << 9));
}

/*
 * Buries a zombie process.
 */
PUBLIC void bury(struct process *proc)
{
	dstrypgdir(proc);
	proc->flags = PROC_FREE;
	proc->state = PROC_DEAD;
	proc->father->nchildren--;
}

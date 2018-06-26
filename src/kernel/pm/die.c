/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2017-2017 Davidson Francis <davidsondfgl@gmail.com>
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

#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <signal.h>

/**
 * @brief Is the system shutting down?
 */
PUBLIC int shutting_down = 0;

/**
 * @brief Kills the current running process.
 * 
 * @param status Exit status.
 */
PUBLIC void die(int status)
{
	struct process *p;
	struct thread *t;

	/* Shall not occour. */
	if (curr_proc == IDLE)
		kpanic("idle process dying");
	
	curr_proc->status = status;
	
	/*
	 * Ignore all signals since, 
	 * process may sleep below.
	 */
	for (unsigned i = 0; i < NR_SIGNALS; i++)
		curr_proc->handlers[i] = SIG_IGN;
	
	/* Close file descriptors. */
	for (unsigned i = 0; i < OPEN_MAX; i++)
		do_close(i);
	
	/* Hangup terminal. */
	if (IS_LEADER(curr_proc) && (curr_proc->tty != NULL_DEV))
		cdev_close(curr_proc->tty);
		
	/* init adopts orphan processes. */
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
	{
		/* Skip invalid processes. */
		if (!IS_VALID(p))
			continue;
		
		if (shutting_down)
			p->father = IDLE;
		else if (p->father == curr_proc)
		{
			p->father = INIT;
			p->father->nchildren++;
			sndsig(INIT, SIGCHLD);
		}
	}
	
	/* init adotps process in the same group. */
	if (curr_proc->pgrp == curr_proc)
	{
		for (p = FIRST_PROC; p <= LAST_PROC; p++)
		{
			/* Skip invalid processes. */
			if (!IS_VALID(p))
				continue;
			
			if (p->pgrp == curr_proc)
			{
				p->pgrp = NULL;
				sndsig(p, SIGHUP);
				sndsig(p, SIGCONT);
			}
		}
	}
	
	/* Detach process memory regions. */
	for (unsigned i = 0; i < NR_PREGIONS; i++)
		detachreg(curr_proc, &curr_proc->pregs[i]);

	/* Force threads regions to detach if this wasn't done previously. */
	t = curr_proc->threads;
	while (t != NULL)
	{
		detachreg(curr_proc, &t->pregs);
		t->state = THRD_TERMINATED;
		t = t->next;
	}

	/* Release root and pwd. */
	inode_put(curr_proc->root);
	inode_put(curr_proc->pwd);
	
	curr_proc->state = PROC_ZOMBIE;
	curr_proc->alarm = 0;



	/* Resets the counter if any. */
	if (curr_thread->pmcs.enable_counters != 0)
		pmc_init();
	
	sndsig(curr_proc->father, SIGCHLD);
	wakeup_join();
	yield();
}

/**
 * @brief Buries a zombie process.
 * 
 * @param proc Process to be buried.
 */
PUBLIC void bury(struct process *proc)
{
	struct thread *t;
	t = proc->threads;
	while (t != NULL)
	{
		detachreg(proc, &t->pregs);
		t->state = THRD_DEAD;
		t = t->next;
	}
	dstrypgdir(proc);
	proc->state = PROC_DEAD;
	proc->father->nchildren--;
	nprocs--;
	wakeup_join();
}

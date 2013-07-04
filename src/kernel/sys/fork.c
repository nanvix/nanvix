/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fork.c - fork() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/paging.h>
#include <nanvix/pm.h>
#include <sys/types.h>
#include <errno.h>

/*
 * Forks the process memory.
 */ 
PRIVATE int fork_mem(struct process *proc)
{
	int i;
	int err;
	struct region *reg;
	struct pregion *preg;
	
	err = crtpgdir(proc);
	
	/* Failed to create process page directory. */
	if (err)
		goto err0;
	
	/* Duplicate attached regions. */
	for (i = 0; i < NR_PREGIONS; i++)
	{
		/* Process region in use. */
		if (curr_proc->pregs[i].type != PREGION_UNUSED)
		{
			preg = &curr_proc->pregs[i];
			
			lockreg(preg->reg);
			reg = dupreg(preg->reg);
			unlockreg(preg->reg);
			
			/* Failed to duplicate region. */
			if (reg == NULL)
				goto err1;
			
			err = attachreg(proc, preg->start, preg->type, reg);
			
			/* Failed to attach region. */
			if (err)
			{
				freereg(reg);
				goto err1;
			}
			
			unlockreg(reg);
		}
	}
	
	return (0);
err1:
	/* Detach attached regions. */
	while (--i >= 0)
	{
		/* Attached region found. */
		if ( proc->pregs[i].type != PREGION_UNUSED)
		{
			/* Detach. */
			preg = &proc->pregs[i];
			lockreg(preg->reg);
			reg = detachreg(proc, preg);
			freereg(reg);
		}
	}
	
err0:
	return (-1);
}

/*
 * Creates a new process.
 */
PUBLIC pid_t sys_fork()
{
	int i;
	int err;
	struct process *proc;

	/* Search for dead process. */
	for (proc = FIRST_PROC; proc <= LAST_PROC; proc++)
	{
		/* Found. */
		if (proc->flags & PROC_FREE)
			goto found;
	}

	kprintf("process table overflow");
	
	return (-EAGAIN);

found:

	/* Mark process as beeing created. */
	proc->flags = PROC_NEW & ~PROC_FREE;

	err = fork_mem(proc);
	
	/* Failed to fork process memory. */
	if (err)
		goto err0;

	proc->intlvl = curr_proc->intlvl;
	proc->received = 0;
	for (i = 0; i < NR_SIGNALS; i++)
		proc->handlers[i] = SIG_DFL;
	proc->uid = curr_proc->uid;
	proc->euid = curr_proc->euid;
	proc->suid = curr_proc->suid;
	proc->gid = curr_proc->gid;
	proc->egid = curr_proc->egid;
	proc->sgid = curr_proc->sgid;
	proc->pid = next_pid++;
	proc->father = curr_proc->pid;
	proc->pgrp = curr_proc->pgrp;
	proc->priority = curr_proc->priority;
	proc->nice = curr_proc->nice;
	proc->alarm = curr_proc->alarm;
	
	sched(proc);
	
	return (proc->pid);

err0:
	proc->flags = PROC_FREE;
	return (-ENOMEM);
}

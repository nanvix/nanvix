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
#include <nanvix/smp.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <sys/types.h>
#include <errno.h>

/*
 * Creates a new process.
 */
PUBLIC pid_t sys_fork(void)
{
	int i;                /* Loop index.     */
	int err;              /* Error?          */
	struct process *proc; /* Process.        */
	struct thread *thrd;  /* New thread.     */
	struct thread *t;     /* Tmp thread.     */
	struct region *reg;   /* Memory region.  */
	struct pregion *preg; /* Process region. */

	/*
	 * Prevent non-privileged user from using the last 
	 * available slot in the process table, so a privileged
	 * user can invoke kill() if something goes wrong.
	 */
	if ((nprocs + 1 >= PROC_MAX) && (!IS_SUPERUSER(curr_proc)))
		return (-EAGAIN);

	/* Search for a free process. */
	for (proc = FIRST_PROC; proc <= LAST_PROC; proc++)
	{
		/* Found. */
		if (!IS_VALID(proc))
			goto found;
	}

	kprintf("process table overflow");

	return (-EAGAIN);

found:
	if ((thrd = get_free_thread()) == NULL)
		return (-EAGAIN);
	
	thrd->state = THRD_READY;
	proc->threads = thrd;

	/* Mark process as beeing created. */
	proc->flags = 1 << PROC_NEW;

	err = crtpgdir(proc);
	
	/* Failed to create process page directory. */
	if (err)
		goto error0;
	
	/*
	 * Duplicate attached regions.
	 * Notice that regions will be attached in the child process
	 * on the same indexes as in the father process.
	 */
	for (i = 0; i < NR_PREGIONS; i++)
	{	
		preg = &curr_proc->pregs[i];
		
		/* Process region not in use. */
		if (preg->reg == NULL)
			continue;	
			
		lockreg(preg->reg);
		reg = dupreg(preg->reg);
		unlockreg(preg->reg);
		
		/* Failed to duplicate region. */
		if (reg == NULL)
			goto error1;
		
		err = attachreg(proc, &proc->pregs[i], preg->start, reg);
		
		/* Failed to attach region. */
		if (err)
		{
			/*
			 * FIXME: region count.
			 */
			kpanic("failed to attach region");
			freereg(reg);
			goto error1;
		}
			
		unlockreg(reg);
	}

	/* Duplicate attached thread region.
	 * There will be only one thread in
	 * the son process according to POSIX */
	preg = &cpus[curr_core].curr_thread->pregs;

	/* Thread region not in use. */
	if (preg->reg == NULL)
		goto dup_done;

	lockreg(preg->reg);
	reg = dupreg(preg->reg);
	unlockreg(preg->reg);

	/* Failed to duplicate region. */
	if (reg == NULL)
		goto error1;

	err = attachreg(proc, &proc->threads->pregs, preg->start, reg);

	/* Failed to attach region. */
	if (err)
	{
		/*
		 * FIXME: region count.
		 */
		kpanic("failed to attach thread region");
		freereg(reg);
		goto error1;
	}

	unlockreg(reg);
dup_done:
	
	/* Initialize process. */
	proc->threads->intlvl = 1;
	proc->received = 0;
	proc->restorer = curr_proc->restorer;
	proc->threads->tid = next_tid++;
	proc->threads->next = NULL;
	proc->threads->retval = NULL;
	proc->threads->flags = 0 << THRD_NEW;

	kmemcpy(&proc->threads->fss, &cpus[curr_core].curr_thread->fss, sizeof(struct fpu));

	for (i = 0; i < NR_SIGNALS; i++)
		proc->handlers[i] = curr_proc->handlers[i];
	proc->threads->irqlvl = cpus[curr_core].curr_thread->irqlvl;
	proc->threads->pmcs.enable_counters = 0;
	proc->pwd = curr_proc->pwd;
	proc->pwd->count++;
	proc->root = curr_proc->root;
	proc->root->count++;
	for (i = 0; i < OPEN_MAX; i++)
	{
		proc->ofiles[i] = curr_proc->ofiles[i];
		
		/* Increment file reference count. */
		if (proc->ofiles[i] != NULL)
			proc->ofiles[i]->count++;
	}
	proc->close = curr_proc->close;
	proc->umask = curr_proc->umask;
	proc->tty = curr_proc->tty;
	proc->status = 0;
	proc->nchildren = 0;
	proc->uid = curr_proc->uid;
	proc->euid = curr_proc->euid;
	proc->suid = curr_proc->suid;
	proc->gid = curr_proc->gid;
	proc->egid = curr_proc->egid;
	proc->sgid = curr_proc->sgid;
	proc->pid = next_pid++;
	proc->pgrp = curr_proc->pgrp;
	proc->father = curr_proc;
	kstrncpy(proc->name, curr_proc->name, NAME_MAX);
	proc->utime = 0;
	proc->ktime = 0;
	proc->cutime = 0;
	proc->cktime = 0;
	proc->priority = curr_proc->priority;
	proc->nice = curr_proc->nice;
	proc->alarm = 0;
	proc->next = NULL;
	proc->chain = NULL;
	proc->size = curr_proc->size;
	t = curr_proc->threads;
	while (t != NULL)
	{
		/* New proc can be smaller than current proc. */
		if (t != cpus[curr_core].curr_thread)
			proc->size -= t->pregs.reg->size;
		t = t->next;
	}



	sched(proc);

	curr_proc->nchildren++;
	
	nprocs++;
	
	return (proc->pid);

error1:
	/* Detach attached regions. */
	while (--i >= 0)
	{
		/* Region not attached. */
		if (proc->pregs[i].reg == NULL)
			continue;
		
		/* Detach. */
		preg = &proc->pregs[i];
		detachreg(proc, preg);
	}
error0:
	dstrypgdir(proc);
	proc->flags = 0;
	return (-ENOMEM);
}

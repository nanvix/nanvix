/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * pm.c - Process manager.
 */

#include <asm/util.h>
#include <i386/i386.h>
#include <nanvix/clock.h>
#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <signal.h>
#include <limits.h>

/* init kstack. */
EXTERN dword_t init_kstack[KSTACK_SIZE/DWORD_SIZE];

/* init page directory. */
EXTERN struct pte init_pgdir[PAGE_SIZE/PTE_SIZE];

/* Process table. */
PUBLIC struct process proctab[PROC_MAX];

/* Current running process. */
PUBLIC struct process *curr_proc = INIT;

/* Next available PID. */
PUBLIC pid_t next_pid = 0;

/*
 * Resets a process to the 'dead' state.
 */
PUBLIC void reset(struct process *proc)
{
	int i;
	
	/* Reset process. */
	proc->flags = PROC_FREE;
	for (i = 0; i < NR_PREGIONS; i++)
		proc->pregs[i].type = PREGION_UNUSED;
	proc->size = 0;
	proc->utime = 0;
	proc->ktime = 0;
	proc->state = PROC_DEAD;
	proc->next = NULL;
	proc->chain = NULL;
}

/*
 * Initializes the process manager.
 */
PUBLIC void pm_init()
{	
	int i;
	struct process *p;
	
	/* Initialize the process table. */
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
		reset(p);
		
	/* Handcraft init process. */
	INIT->kesp = 0;
	INIT->cr3 = (dword_t)init_pgdir;
	INIT->intlvl = 1;
	INIT->flags = 0;
	INIT->received = 0;
	INIT->kstack = init_kstack;
	for (i = 0; i < NR_SIGNALS; i++)
		INIT->handlers[i] = SIG_DFL;
	INIT->pgdir = init_pgdir;
	for (i = 0; i < NR_PREGIONS; i++)
	{
		INIT->pregs[i].type = PREGION_UNUSED;
		INIT->pregs[i].start = 0;
		INIT->pregs[i].reg = NULL;
	}
	INIT->size = 0;
	INIT->uid = SUPERUSER;
	INIT->euid = SUPERUSER;
	INIT->suid = SUPERUSER;
	INIT->gid = SUPERGROUP;
	INIT->egid = SUPERGROUP;
	INIT->sgid = SUPERGROUP;
	INIT->pid = next_pid++;
	INIT->father = -1;
	INIT->pgrp = INIT->pid;
	INIT->utime = 0;
	INIT->ktime = 0;
	INIT->state = PROC_RUNNING;
	INIT->counter = PROC_QUANTUM;
	INIT->priority = PRIO_INIT;
	INIT->nice = NZERO;
	INIT->alarm = 0;
	INIT->next = NULL;
	INIT->chain = NULL;
	
	clock_init(CLOCK_FREQ);
	
	enable_interrupts();
	
	kprintf("init process spawned");
}

/*
 * Terminates a process.
 */
PUBLIC void terminate(struct process *proc, int err)
{
	kprintf("terminating process %d with error code %d", proc->pid, err);
	
	while (1)
		yield();
}

/*
 * Aborts the execution of a process.
 */
PUBLIC void abort(struct process *proc)
{
	UNUSED(proc);
	
	kprintf("aborting process %d", proc->pid);
	
	while (1)
		yield();
}

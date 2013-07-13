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

/* init stuff. */
EXTERN char init_kstack[]; /* Kernel stack.   */
EXTERN struct pte init_pgdir[];  /* Page directory. */

/* Process table. */
PUBLIC struct process proctab[PROC_MAX];

/* Current running process. */
PUBLIC struct process *curr_proc = INIT;

/* Next available PID. */
PUBLIC pid_t next_pid = 1;

/*
 * Initializes the process manager.
 */
PUBLIC void pm_init()
{	
	int i;
	struct process *p;
	
	/* Initialize the process table. */
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
		p->flags = PROC_FREE;
		
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
	INIT->status = 0;
	INIT->nchildren = 0;
	INIT->uid = SUPERUSER;
	INIT->euid = SUPERUSER;
	INIT->suid = SUPERUSER;
	INIT->gid = SUPERGROUP;
	INIT->egid = SUPERGROUP;
	INIT->sgid = SUPERGROUP;
	INIT->pid = next_pid++;
	INIT->father = NULL;
	INIT->pgrp = NULL;
	INIT->utime = 0;
	INIT->ktime = 0;
	INIT->state = PROC_RUNNING;
	INIT->counter = PROC_QUANTUM;
	INIT->priority = PRIO_USER;
	INIT->nice = 2*NZERO -1;
	INIT->alarm = 0;
	INIT->next = NULL;
	INIT->chain = NULL;
	
	clock_init(CLOCK_FREQ);
	
	enable_interrupts();
}

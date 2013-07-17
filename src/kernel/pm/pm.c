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
PUBLIC char init_kstack[KSTACK_SIZE]; /* Kernel stack.   */
EXTERN struct pte init_pgdir[];       /* Page directory. */

/* Process table. */
PUBLIC struct process proctab[PROC_MAX];

/* Current running process. */
PUBLIC struct process *curr_proc = IDLE;

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
	IDLE->kesp = 0;
	IDLE->cr3 = (dword_t)init_pgdir;
	IDLE->intlvl = 1;
	IDLE->flags = 0;
	IDLE->received = 0;
	IDLE->kstack = init_kstack;
	for (i = 0; i < NR_SIGNALS; i++)
		IDLE->handlers[i] = SIG_DFL;
	IDLE->pgdir = init_pgdir;
	for (i = 0; i < NR_PREGIONS; i++)
	{
		IDLE->pregs[i].type = PREGION_UNUSED;
		IDLE->pregs[i].start = 0;
		IDLE->pregs[i].reg = NULL;
	}
	IDLE->size = 0;
	IDLE->status = 0;
	IDLE->nchildren = 0;
	IDLE->uid = SUPERUSER;
	IDLE->euid = SUPERUSER;
	IDLE->suid = SUPERUSER;
	IDLE->gid = SUPERGROUP;
	IDLE->egid = SUPERGROUP;
	IDLE->sgid = SUPERGROUP;
	IDLE->pid = next_pid++;
	IDLE->father = NULL;
	IDLE->pgrp = NULL;
	IDLE->utime = 0;
	IDLE->ktime = 0;
	IDLE->state = PROC_RUNNING;
	IDLE->counter = PROC_QUANTUM;
	IDLE->priority = PRIO_USER;
	IDLE->nice = 2*NZERO -1;
	IDLE->alarm = 0;
	IDLE->next = NULL;
	IDLE->chain = NULL;
	
	clock_init(CLOCK_FREQ);
	
	enable_interrupts();
}

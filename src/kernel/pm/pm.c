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
#include <nanvix/dev.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <sys/stat.h>
#include <signal.h>
#include <limits.h>

/* init stuff. */
PUBLIC char init_kstack[KSTACK_SIZE]; /* Kernel stack.   */
EXTERN struct pde init_pgdir[];       /* Page directory. */

/* Process table. */
PUBLIC struct process proctab[PROC_MAX];

/* Current running process. */
PUBLIC struct process *curr_proc = IDLE;

/* Next available PID. */
PUBLIC pid_t next_pid = 0;

/* Current number of process in the system. */
PUBLIC int nprocs = 0;

/*
 * Initializes the process manager.
 */
PUBLIC void pm_init(void)
{	
	int i;             /* Loop index.      */
	struct process *p; /* Working process. */
	
	/* Initialize the process table. */
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
		p->flags = PROC_FREE;
		
	/* Handcraft init process. */
	IDLE->cr3 = (dword_t)init_pgdir;
	IDLE->intlvl = 1;
	IDLE->flags = 0;
	IDLE->received = 0;
	IDLE->kstack = init_kstack;
	for (i = 0; i < NR_SIGNALS; i++)
		IDLE->handlers[i] = SIG_DFL;
	IDLE->pgdir = init_pgdir;
	for (i = 0; i < NR_PREGIONS; i++)
		IDLE->pregs[i].reg = NULL;
	IDLE->size = 0;
	IDLE->pwd = root;
	IDLE->root = root;
	root->count += 2;
	for (i = 0; i < OPEN_MAX; i++)
		IDLE->ofiles[i] = NULL;
	IDLE->close = 0;
	IDLE->umask = S_IXUSR | S_IWGRP | S_IXGRP | S_IWOTH | S_IXOTH;
	IDLE->tty = NULL_DEV;
	IDLE->status = 0;
	IDLE->nchildren = 0;
	IDLE->uid = SUPERUSER;
	IDLE->euid = SUPERUSER;
	IDLE->suid = SUPERUSER;
	IDLE->gid = SUPERGROUP;
	IDLE->egid = SUPERGROUP;
	IDLE->sgid = SUPERGROUP;
	IDLE->pid = next_pid++;
	IDLE->pgrp = IDLE;
	IDLE->father = NULL;
	IDLE->utime = 0;
	IDLE->ktime = 0;
	IDLE->cutime = 0;
	IDLE->cktime = 0;
	IDLE->state = PROC_RUNNING;
	IDLE->counter = PROC_QUANTUM;
	IDLE->priority = PRIO_USER;
	IDLE->nice = NZERO;
	IDLE->alarm = 0;
	IDLE->next = NULL;
	IDLE->chain = NULL;
	
	nprocs++;
	
	clock_init(CLOCK_FREQ);
	
	enable_interrupts();
}

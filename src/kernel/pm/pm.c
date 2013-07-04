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

/* init kstack. */
EXTERN dword_t init_kstack[KSTACK_SIZE/DWORD_SIZE];

/* init page directory. */
EXTERN struct pte init_pgdir[PAGE_SIZE/PTE_SIZE];

/* Process table. */
PUBLIC struct process proctab[PROC_MAX];

/* Current running process. */
PUBLIC struct process *curr_proc = INIT;

/*
 * Initializes the process manager.
 */
PUBLIC void pm_init()
{	
	struct process *p;
	
	/* Initialize the process table. */
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
	{
		p->flags = PROC_FREE;
		p->state = PROC_DEAD;
	}
		
	/* Handcraft init process. */
	INIT->kesp = (dword_t)init_kstack + KSTACK_SIZE;
	INIT->cr3 = (dword_t)init_pgdir;
	INIT->intlvl = 1;
	INIT->kstack = init_kstack;
	INIT->pgdir = init_pgdir;
	INIT->state = PROC_RUNNING;
	INIT->priority = PRIO_INIT;
	INIT->nice = -100;
	INIT->next = NULL;
	
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

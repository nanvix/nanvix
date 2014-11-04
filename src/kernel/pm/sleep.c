/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * ipc.c - Inter-process communication
 */

#include <asm/util.h>
#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/pm.h>

/*
 * Sleep queue for idle process.
 */
PRIVATE struct process **idle_queue = NULL;

/*
 * Puts a process to sleep in a sleeping chain.
 */
PUBLIC void sleep(struct process **chain, int priority)
{	
	/*
	 * Idle process trying to sleep,
	 * so let's do some busy waiting.
	 */
	if (curr_proc == IDLE)
	{
		kprintf("pm: idle process sleeping");
		
		/* Busy wait. */
		idle_queue = chain;
		enable_interrupts();
		while (idle_queue == chain)
			/* noop*/ ;
		
		return;
	}

	/* Interruptible sleep and pending signal. */
	if ((priority >= 0) && (curr_proc->received))
		return;
		
	/* Insert process in the sleeping chain. */
	curr_proc->next = *chain;
	*chain = curr_proc;
	
	/* Put process to sleep. */
	curr_proc->state = (priority >= 0) ? PROC_WAITING : PROC_SLEEPING;
	curr_proc->priority = priority;
	curr_proc->chain = chain;
	
	yield();
}
	
/*
 * Wakes up all process that are sleeping in a chain.
 */
PUBLIC void wakeup(struct process **chain)
{	
	/* Wakeup idle process. */
	if (idle_queue == chain)
	{
		idle_queue = NULL;
		return;
	}
	
	/* Wakeup sleeping processes. */
	while (*chain != NULL)
	{
		sched(*chain);
		*chain = (*chain)->next;
	}
}

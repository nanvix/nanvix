/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * ipc.c - Inter-process communication
 */

#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/pm.h>

/*
 * Puts a process to sleep in a sleeping chain.
 */
PUBLIC void sleep(struct process **chain, int priority)
{	
	/* Process 0 cannot sleep. */
	if (curr_proc == IDLE)
		kpanic("idle process trying to sleep");

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
	/* Wakeup sleeping processes. */
	while (*chain != NULL)
	{
		sched(*chain);
		*chain = (*chain)->next;
	}
}

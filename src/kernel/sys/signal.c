/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * signal.c - pause() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <signal.h>

/*
 * 
 */
PUBLIC sighandler_t sys_signal(int signum, sighandler_t handler, void (*restorer)(void))
{
	sighandler_t old_handler;
	
	/* Invalid signal. */
	if ((signum < 0) || (signum >= NR_SIGNALS))
		return (NULL);
	
	/* Cannot be caught or ignored. */
	if ((signum == SIGKILL) || (signum == SIGSTOP))
		return (NULL);
	
	/* Handler is not valid. */
	if (!chkmem((addr_t)handler, CHKMEM_FUNCTION))
		return (NULL);
	
	/* Restorer is not valid. */
	if (!chkmem((addr_t)restorer, CHKMEM_FUNCTION))
		return (NULL);
	
	/* Set signal handler. */
	old_handler = curr_proc->handlers[signum];
	curr_proc->handlers[signum] = handler;
	curr_proc->restorers[signum] = restorer;
	
	/*
	 * Pending signals are discarded to avoid any
	 * race conditions.
	 */
	curr_proc->received &= ~(1 << signum);
	
	return (old_handler);
}

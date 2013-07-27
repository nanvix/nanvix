/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/signal.c> - signal() system call implementation.
 * 
 * XXX: check what happens if signal() is executed on a pending signal.
 */

#include <nanvix/const.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <signal.h>

/*
 * Manages signals.
 */
PUBLIC sighandler_t sys_signal(int sig, sighandler_t func, sigrestorer_t restorer)
{
	sighandler_t old_func;
	
	/* Invalid signal. */
	if ((sig < 0) || (sig >= NR_SIGNALS))
		return (SIG_ERR);
	
	/* Cannot be caught or ignored. */
	if ((sig == SIGKILL) || (sig == SIGSTOP))
		return (SIG_ERR);
	
	/* Check handler function. */
	if ((func != SIG_DFL) && (func != SIG_IGN))
	{
		/* Handler is not valid. */
		if ((!chkmem((addr_t)func, CHK_FUNCTION)))
			return (SIG_ERR);
	}
	
	/* Restorer is not valid. */
	if ((chkmem((addr_t)restorer, CHK_FUNCTION) != OK))
		return (SIG_ERR);
	
	/* Set signal handler. */
	old_func = curr_proc->handlers[sig];
	curr_proc->handlers[sig] = func;
	curr_proc->restorers[sig] = restorer;
	
	return (old_func);
}

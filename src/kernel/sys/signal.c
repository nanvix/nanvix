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
 * Manages signal handling.
 */
PUBLIC sighandler_t sys_signal(int sig, sighandler_t func)
{
	sighandler_t old_func;
	
	/* Invalid signal. */
	if ((sig < 0) || (sig >= NR_SIGNALS))
		return (SIG_ERR);
	
	/* Cannot be caught or ignored. */
	if ((sig == SIGKILL) || (sig == SIGSTOP))
		return (SIG_ERR);
	
	/* Set signal handler. */
	old_func = curr_proc->handlers[sig];
	curr_proc->handlers[sig] = func;
	
	return (old_func);
}

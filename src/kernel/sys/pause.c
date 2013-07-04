/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * pause.c - pause() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <errno.h>
#include <signal.h>

/* Pause sleeping chain. */
PRIVATE struct process *chain = NULL;

/*
 * Suspend the calling process until a signal is received.
 */
PUBLIC int sys_pause()
{
	int i;
	
	/* Susped process. */
	while (1)
	{
		sleep(&chain, PRIO_USER);
	
		/* Check which signal awakened us. */
		for (i = 0; i < NR_SIGNALS; i++)
		{
			/* This signal. */
			if (curr_proc->received & (1 << i))
			{
				/* Catch signal. */
				if (curr_proc->handlers[i] != SIG_DFL)
					goto awaken;
			
				/* Terminate process. */
				if (sig_default[i] == (sighandler_t)&terminate)
					goto awaken;
					
				/* Clear signal. */
				curr_proc->received &= ~(1 << i);
			}
		}
	}

awaken:

	return (-EINTR);
}

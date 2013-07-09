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
	/* Susped process. */
	while (1)
	{
		sleep(&chain, PRIO_USER);
		
		/* Awaken on signal. */
		if (issig())
			goto awaken;
	}

awaken:

	return (-EINTR);
}

/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * wait.c - wait() system call implementation.
 * 
 * TODO: improve comments.
 * 
 * XXX: test wait(NULL).
 * XXX: test wait(&whatever).
 * XXX: test EINVAL.
 * XXX: test ECHILD when the process has no children.
 * XXX: test ECHILD when SIGCHLD is set to SIG_IGN.
 * XXX: test EINTR.
 */

#include <nanvix/const.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <sys/types.h>
#include <errno.h>

/* Sleeping chain. */
PRIVATE struct process *chain = NULL;

/*
 * Waits for a child process to terminate.
 */
PUBLIC pid_t sys_wait(int *stat_loc)
{
	pid_t pid;
	struct process *p;

	/* Has no permissions to write at stat_loc. */
	if ((stat_loc != NULL) && (!chkmem((addr_t)stat_loc, CHKMEM_CHUNK, 1, sizeof(int))))
		return (-EINVAL);

repeat:

	/* Nobody to wait for. */
	if (curr_proc->nchildren == 0)
		return (-ECHILD);

	/* Look for child processes. */
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
	{
		 /* Found. */
		if (p->father == curr_proc)
		{
			/* Task has already terminated. */
			if (p->state == PROC_ZOMBIE)
			{
				/* Get exit code. */
				if (stat_loc != NULL)
					*stat_loc = p->status;

				pid = p->pid;

				/* Bury child process. */
				bury(p);
				
				/* Return task pid. */
				return (pid);
			}
		}
	}
	
	sleep(&chain, PRIO_USER);
	
	if (issig() == SIGCHLD)
		goto repeat;
		
	return (-EINTR);
}

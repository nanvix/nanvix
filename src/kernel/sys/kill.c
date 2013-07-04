/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * kill.c - kill() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <sys/types.h>
#include <errno.h>
#include <signal.h>

/*
 * Asserts if the process p1 is authorized to send a signal to the process p2.
 */
#define AUTHORIZED(p1, p2, s)                          \
	((IS_SUPERUSER(p1)) ||                             \
	 ((s == SIGCONT) && (p1->pgrp == p2->pgrp)) ||     \
	 (p1->uid == p2->uid)  || (p1->euid == p2->uid) || \
	 (p1->uid == p2->suid) || (p1->euid == p2->suid))  \

/*
 * Sends a signal to a process or a process group.
 */
PUBLIC int sys_kill(pid_t pid, int sig)
{
	int err_eperm;
	int err_esrch;
	struct process *p;
	
	/* Invalid signal. */
	if ((sig < 0) || (sig > NR_SIGNALS - 1))
		return (-EINVAL);
	
	err_eperm = EPERM;
	err_esrch = ESRCH;
	
	/* Send signal to process. */
	if (pid > 0)
	{
		/* Search for process. */
		for (p = FIRST_PROC; p <= LAST_PROC; p++)
		{
			err_esrch = 0;
			
			/* Found. */
			if (pid == p->pid)
			{
				if (AUTHORIZED(curr_proc, p, sig))
				{
					err_eperm = 0;
					sndsig(p, sig);
				}
			}
		}
	}
	
	/* Send signal to process group. */
	else if (pid == 0)
	{
		/* Search for processes. */
		for (p = FIRST_PROC; p <= LAST_PROC; p++)
		{
			/* Found. */
			if (curr_proc->pid == p->pgrp)
			{
				err_esrch = 0;
			
				if (AUTHORIZED(curr_proc, p, sig))
				{
					err_eperm = 0;
					sndsig(p, sig);
				}
			}
		}
	}
	
	/* Send signal to all processes. */
	else if (pid == -1)
	{
		err_esrch = 0;
		
		/* Search for processes. */
		for (p = FIRST_PROC; p <= LAST_PROC; p++)
		{
			
			if (AUTHORIZED(curr_proc, p, sig))
			{
				err_eperm = 0;
				sndsig(p, sig);
			}
		}
	}
	
	/* Send signal to absolute proces group. */
	else
	{
		/* Search for processes. */
		for (p = FIRST_PROC; p <= LAST_PROC; p++)
		{			
			/* Found. */
			if (p->pgrp == -pid)
			{
				err_esrch = 0;
			
				if (AUTHORIZED(curr_proc, p, sig))
				{
					err_eperm = 0;
					sndsig(p, sig);
				}
			}
		}
	}
	
	return ((err_esrch) ? err_esrch : err_eperm );
}

/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * signal.c - Signals.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>
#include <signal.h>
#include "pm.h"

/*
 * Asserts if a process is ignoring a signal.
 */
#define IGNORING(p, sig)                                              \
		((p->handlers[sig] == SIG_IGN) ||                             \
		 ((p->handlers[sig] == SIG_DFL) && (sigdfl[sig] == SIG_IGN))) \

/*
 * Asserts if a process is catching a signal.
 */
#define CATCHING(p, sig)                                             \
	((p->handlers[sig] != SIG_DFL) && (p->handlers[sig] != SIG_IGN)) \

/* Default signal handlers. */
PRIVATE const sighandler_t sigdfl[NR_SIGNALS] = {
	NULL,                     /* SIGNULL */ (sighandler_t)&terminate, /* SIGKILL */ 
	(sighandler_t)&stop,      /* SIGSTOP */ SIG_IGN,                  /* SIGURG  */
	(sighandler_t)&abort,     /* SIGABRT */ (sighandler_t)&abort,     /* SIGBUS  */
	SIG_IGN,                  /* SIGCHLD */ NULL,                     /* SIGCONT */
	(sighandler_t)&terminate, /* SIGFPE  */ (sighandler_t)&terminate, /* SIGHUP  */
	(sighandler_t)&abort,     /* SIGILL  */ (sighandler_t)&terminate, /* SIGINT  */
	(sighandler_t)&abort,     /* SIGPIPE */ (sighandler_t)&abort,     /* SIGQUIT */
	(sighandler_t)&abort,     /* SIGSEGV */ (sighandler_t)&terminate, /* SIGTERM */
	(sighandler_t)&stop,      /* SIGTSTP */ (sighandler_t)&stop,      /* SIGTTIN */
	(sighandler_t)&stop,      /* SIGTTOU */ (sighandler_t)&terminate, /* SIGALRM */
	(sighandler_t)&terminate, /* SIGUSR1 */ (sighandler_t)&terminate, /* SIGUSR2 */
	(sighandler_t)&abort,     /* SIGTRAP */
};

/*
 * Sends a signal to a process.
 */
PUBLIC void sndsig(struct process *proc, int sig)
{
	struct process *p;
	
	/* Ignore SIGNULL. */
	if (sig == SIGNULL)
		return;
	
	/* Set signal flag. */
	proc->received |= (1 << sig);
	
	/* 
	 * SIGCONT is somewhat special. The receiving process is resumed even if
	 * SIGCONT is being ignored otherwise it could stop forever.
	 */
	if (sig == SIGCONT)
	{
		resume(proc);
		
		/* 
		 * The process is not catching SIGCONT and the default action for this
		 * signal has already been taken.
		 */
		if (!CATCHING(proc, sig))
		{
			proc->received &= ~(1 << sig);
			return;
		}
	}
	
	/* Wake up process. */
	if (proc->state == PROC_WAITING)
	{
		/* Remove process from the sleeping chain. */
		if (proc == *proc->chain)
			*proc->chain = proc->next;
		else
		{
			for (p = *proc->chain; p->next != proc; p = p->next);
				/* NOOP */ ;
			p->next = proc->next;
		}
		
		/* Re-schedule process. */
		sched(proc);
	}
}

/*
 * Checks if the current process has a pending signal.
 */
PUBLIC int issig(void)
{
	int i;
	struct process *p;

	/* Find a signal that is pending and not being ignored. */
	for (i = 1; i < NR_SIGNALS; i++)
	{
		/* Found. */
		if (curr_proc->received & (1 << i))
		{
			/*
			 * SIGCHLD is somewhat special. If the current process has set SIG_IGN
			 * to handle SIGCHLD, the child processes should not become zombie 
			 * processes, meaning that they are buried the soonest possible (here). 
			 * However, if the current process is catching SIGCHLD or handling it
			 * with the default handler (SIG_DFL), child processes do become zombie
			 * processes and they are buried in wait().
			 */
			if (i == SIGCHLD)
			{
				/* The current process has set SIG_IGN to handle SIGCHLD. */
				if (curr_proc->handlers[SIGCHLD] == SIG_IGN)
				{		
					/* Clear signal flag. */
					curr_proc->received &= ~(1 << i);
				
					/* Bury zombie child processes. */
					for (p = FIRST_PROC; p <= LAST_PROC; p++)
					{
						/* Zombie child process found. */
						if ((p->father == curr_proc) && (p->state == PROC_ZOMBIE))
							bury(p);
					}
					
					/*
					 * The current process still have child processes, so try 
					 * to find another signal that is pending and not being ignored.
					 */
					if (curr_proc->nchildren)
						continue;
					
					return (SIGNULL);
				}
				
				/* 
				 * Clear the signal flag for SIGCHLD since the current process is
				 * not catching this signal and the default action is to ignore it.
				 */
				else if (curr_proc->handlers[SIGCHLD] == SIG_DFL)
				{
					curr_proc->received &= ~(1 << i);
					return (SIGNULL);
				}
				
				/* wait() will bury zombie child process (if any). */
				return (SIGCHLD);
			}
			
			/* Found. */
			else if (!IGNORING(curr_proc, i))
				return (i);

			/* Clear signal flag. */
			curr_proc->received &= ~(1 << i);
		}
	}
	
	/* There is no pending signal that is not being ignored. */
	return (SIGNULL);
}

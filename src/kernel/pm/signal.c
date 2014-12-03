/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * This file is part of Nanvix.
 *
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/pm.h>
#include <signal.h>

/* Forward definitions. */
PRIVATE void abort(int);
PRIVATE void resume(struct process *);
PRIVATE void stop(void);
PRIVATE void terminate(int);
	
/**
 * @brief Asserts if a process is ignoring a signal.
 * 
 * @param p   Process to be inspected.
 * @param sig Signal to be checked.
 * 
 * @returns True if the process is ignoring the signal, and false otherwise.
 */
#define IGNORING(p, sig)                                                \
		(((p)->handlers[sig] == SIG_IGN) ||                             \
		 (((p)->handlers[sig] == SIG_DFL) && (sigdfl[sig] == SIG_IGN))) \

/**
 * @brief Asserts if a process is catching a signal.
 * 
 * @param p   Process to be inspected.
 * @param sig Signal to be checked.
 */
#define CATCHING(p, sig)                                                 \
	(((p)->handlers[sig] != SIG_DFL) && ((p)->handlers[sig] != SIG_IGN)) \

/**
 * @brief Default signal handlers.
 */
PRIVATE const sighandler_t sigdfl[NR_SIGNALS] = {
	NULL,                     /* SIGNULL */
	(sighandler_t)&terminate, /* SIGKILL */ 
	(sighandler_t)&stop,      /* SIGSTOP */
	SIG_IGN,                  /* SIGURG  */
	(sighandler_t)&abort,     /* SIGABRT */
	(sighandler_t)&abort,     /* SIGBUS  */
	SIG_IGN,                  /* SIGCHLD */
	NULL,                     /* SIGCONT */
	(sighandler_t)&terminate, /* SIGFPE  */
	(sighandler_t)&terminate, /* SIGHUP  */
	(sighandler_t)&abort,     /* SIGILL  */
	(sighandler_t)&terminate, /* SIGINT  */
	(sighandler_t)&abort,     /* SIGPIPE */
	(sighandler_t)&abort,     /* SIGQUIT */
	(sighandler_t)&abort,     /* SIGSEGV */
	(sighandler_t)&terminate, /* SIGTERM */
	(sighandler_t)&stop,      /* SIGTSTP */
	(sighandler_t)&stop,      /* SIGTTIN */
	(sighandler_t)&stop,      /* SIGTTOU */
	(sighandler_t)&terminate, /* SIGALRM */
	(sighandler_t)&terminate, /* SIGUSR1 */
	(sighandler_t)&terminate, /* SIGUSR2 */
	(sighandler_t)&abort,     /* SIGTRAP */
};

/**
 * @brief Terminates the current running process.
 * 
 * @param sig Signal number that caused termination.
 */
PRIVATE void terminate(int sig)
{
	die(((sig & 0xff) << 16) | (1 << 9));
}

/**
 * @brief Aborts the current running process.
 * 
 * @param sig Signal number that caused abortion.
 */
PRIVATE void abort(int sig)
{
	die(((sig & 0xff) << 16) | (1 << 9));
}

/**
 * @brief Stops the current running process.
 */
PRIVATE void stop(void)
{
	curr_proc->state = PROC_STOPPED;
	yield();
}

/**
 * @brief Resumes a process.
 * 
 * @param proc Process to be resumed.
 * 
 * @note The process must stopped to be resumed.
 */
PRIVATE void resume(struct process *proc)
{	
	/* Resume only if process has stopped. */
	if (proc->state == PROC_STOPPED)
		sched(proc);
}

/**
 * @brief Sends a signal to a process.
 * 
 * @param proc Process to send the signal.
 * @param sig  Signal to be sent.
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
	 * SIGCONT is somewhat special. The receiving process
	 * is resumed even if SIGCONT is being ignored,
	 * otherwise it could stop forever.
	 */
	if (sig == SIGCONT)
	{
		resume(proc);
		
		/* 
		 * The process is not catching SIGCONT and the
		 * default action for this signal has just been
		 * taken.
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
		if (proc == *proc->chain)
			*proc->chain = proc->next;
		else
		{
			for (p = *proc->chain; p->next != proc; p = p->next);
				noop() ;
			p->next = proc->next;
		}
		sched(proc);
	}
}

/**
 * @brief Checks if the current process has a pending signal.
 * 
 * @returns The number of the signal that is pending. If no signal is pending,
 *          SIGNULL is returned.
 */
PUBLIC int issig(void)
{
	int i;
	struct process *p;

	/* Find a pending signal that is not being ignored. */
	for (i = 1; i < NR_SIGNALS; i++)
	{
		/* Skip. */
		if (!(curr_proc->received & (1 << i)))
			continue;
		
		/*
		 * SIGCHLD is somewhat special. If the current process
		 * has set SIG_IGN to handle SIGCHLD, the child processes
		 * should not become zombie  processes. However, if
		 * the current process is catching SIGCHLD or handling it
		 * with the default handler (SIG_DFL), child processes do
		 * become zombie processes and they are buried in wait().
		 */
		if (i == SIGCHLD)
		{
			/* The current process has set SIG_IGN to handle SIGCHLD. */
			if (curr_proc->handlers[SIGCHLD] == SIG_IGN)
			{
				curr_proc->received &= ~(1 << i);
			
				/* Bury zombie child processes. */
				for (p = FIRST_PROC; p <= LAST_PROC; p++)
				{
					if ((p->father == curr_proc) && (p->state==PROC_ZOMBIE))
						bury(p);
				}
				
				/*
				 * The current process still have child processes,
				 * so try to find another signal that is pending
				 * and not being ignored.
				 */
				if (curr_proc->nchildren)
					continue;
				
				return (SIGNULL);
			}
			
			/* 
			 * Clear the signal flag for SIGCHLD since the current
			 * process is not catching this signal and the default
			 * action is to ignore it.
			 */
			else if (curr_proc->handlers[SIGCHLD] == SIG_DFL)
			{
				curr_proc->received &= ~(1 << i);
				return (SIGNULL);
			}
			
			return (SIGCHLD);
		}
		
		/* Found. */
		else if (!IGNORING(curr_proc, i))
			return (i);
		
		/* Clear signal flag. */
		curr_proc->received &= ~(1 << i);
	}
	
	return (SIGNULL);
}

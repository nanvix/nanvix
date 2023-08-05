/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
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
PRIVATE void _abort(int);
PRIVATE void _terminate(int);

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
PUBLIC const sighandler_t sigdfl[NR_SIGNALS] = {
	SIG_IGN,                   /* SIGNULL */
	(sighandler_t)&_terminate, /* SIGKILL */
	(sighandler_t)&stop,       /* SIGSTOP */
	SIG_IGN,                   /* SIGURG  */
	(sighandler_t)&_abort,     /* SIGABRT */
	(sighandler_t)&_abort,     /* SIGBUS  */
	SIG_IGN,                   /* SIGCHLD */
	NULL,                      /* SIGCONT */
	(sighandler_t)&_terminate, /* SIGFPE  */
	(sighandler_t)&_terminate, /* SIGHUP  */
	(sighandler_t)&_abort,     /* SIGILL  */
	(sighandler_t)&_terminate, /* SIGINT  */
	(sighandler_t)&_abort,     /* SIGPIPE */
	(sighandler_t)&_abort,     /* SIGQUIT */
	(sighandler_t)&_abort,     /* SIGSEGV */
	(sighandler_t)&_terminate, /* SIGTERM */
	(sighandler_t)&stop,       /* SIGTSTP */
	(sighandler_t)&stop,       /* SIGTTIN */
	(sighandler_t)&stop,       /* SIGTTOU */
	(sighandler_t)&_terminate, /* SIGALRM */
	(sighandler_t)&_terminate, /* SIGUSR1 */
	(sighandler_t)&_terminate, /* SIGUSR2 */
	(sighandler_t)&_abort,     /* SIGTRAP */
};

/**
 * @brief Terminates the current running process.
 *
 * @param sig Signal number that caused termination.
 */
PRIVATE void _terminate(int sig)
{
	die(((sig & 0xff) << 16) | (1 << 9));
}

/**
 * @brief Aborts the current running process.
 *
 * @param sig Signal number that caused abortion.
 */
PRIVATE void _abort(int sig)
{
	kprintf("core dumped");
	die(((sig & 0xff) << 16) | (1 << 9));
}

/**
 * @brief Sends a signal to a process.
 *
 * @param proc Process to send the signal.
 * @param sig  Signal to be sent.
 */
PUBLIC void sndsig(struct process *proc, int sig)
{
	/*
	 * SIGCHLD and SIGCONT are somewhat special. The receiving
	 * process is resumed even if these signals are being ignored,
	 * otherwise caos might follow.
	 */
	if ((sig != SIGCHLD) && (sig != SIGCONT))
	{
		if (IGNORING(proc, sig))
			return;
	}

	/* Set signal flag. */
	proc->received |= (1 << sig);

	/* Wake up process. */
	if (proc->state == PROC_WAITING)
	{
		if (proc == *proc->chain)
			*proc->chain = proc->next;
		else
		{
			struct process *p;
			for (p = *proc->chain; p->next != proc; p = p->next)
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
	int ret;

	ret = SIGNULL;

	/* Find a pending signal that is not being ignored. */
	for (int i = 1; i < NR_SIGNALS; i++)
	{
		/* Skip. */
		if (!(curr_proc->received & (1 << i)))
			continue;

		/*
		 * Default action for SIGCONT has already been taken. If the current
		 * process is not catching SIGCONT, we have to clear the signal flag.
		 */
		if (i == SIGCONT)
		{
			if (CATCHING(curr_proc, i))
				return (SIGCONT);

			ret = SIGCONT;
			curr_proc->received &= ~(1 << i);
		}

		/*
		 * SIGCHLD is somewhat special. If the current process
		 * has set SIG_IGN to handle SIGCHLD, the child processes
		 * should not become zombie  processes. However, if
		 * the current process is catching SIGCHLD or handling it
		 * with the default handler (SIG_DFL), child processes do
		 * become zombie processes and they are buried in wait().
		 */
		else if (i == SIGCHLD)
		{
			/* The current process has set SIG_IGN to handle SIGCHLD. */
			if (curr_proc->handlers[SIGCHLD] == SIG_IGN)
			{
				curr_proc->received &= ~(1 << i);

				/* Bury zombie child processes. */
				for (struct process *p = FIRST_PROC; p <= LAST_PROC; p++)
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

		/*
		 * Check if the default action for this signal
		 * is to stop execution. If so, we do it now.
		 */
		if (curr_proc->handlers[i] == SIG_DFL)
		{
			if (sigdfl[i] == (sighandler_t)&stop)
			{
				curr_proc->received &= ~(1 << i);
				stop();
				return (SIGNULL);
			}
		}

		return (i);
	}

	return (ret);
}

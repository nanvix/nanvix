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
#ifndef SIGNAL_H_
#define SIGNAL_H_

	#define NR_SIGNALS 23

	/* Signals. */
	#define SIGNULL  0
	#define SIGKILL  1
	#define SIGSTOP  2
	#define SIGURG   3
	#define SIGABRT  4
	#define SIGBUS   5
	#define SIGCHLD  6
	#define SIGCONT  7
	#define SIGFPE   8
	#define SIGHUP   9
	#define SIGILL  10
	#define SIGINT  11
	#define SIGPIPE 12
	#define SIGQUIT 13
	#define SIGSEGV 14
	#define SIGTERM 15
	#define SIGTSTP 16
	#define SIGTTIN 17
	#define SIGTTOU 18
	#define SIGALRM 19
	#define SIGUSR1 20
	#define SIGUSR2 21
	#define SIGTRAP 22

	/* Handlers. */
	#define _SIG_DFL 1
	#define SIG_DFL (sighandler_t)(_SIG_DFL)
    #define _SIG_IGN 2
    #define SIG_IGN (sighandler_t)(_SIG_IGN)

    /* Other constants. */
    #define _SIG_ERR 0
    #define SIG_ERR (sighandler_t)(_SIG_ERR)

#ifndef _ASM_FILE_

	#include <sys/types.h>

	/* Types. */
	typedef void (*sighandler_t)(int);

	/* Function prototypes. */
	extern sighandler_t signal(int sig, sighandler_t func);
	extern int kill(pid_t pid, int sig);

#endif /* _ASM_FILE_ */

#endif /* SIGNAL_H_ */

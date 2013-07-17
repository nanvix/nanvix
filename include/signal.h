/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * signal.h - Signals library header.
 */

#ifndef SIGNAL_H_
#define SIGNAL_H_

	/* Number of signals. */
	#define NR_SIGNALS 23

	/* Sinals. */
	#define SIGNULL    0 /* Null signal.                                    */
	#define SIGKILL    1 /* Kill (cannot be caught or ignored).             */
	#define SIGSTOP    2 /* Stop executing (cannot be caught or ignored).   */
	#define SIGURG     3 /* High bandwidth data is available at a socket.   */
	#define SIGABRT    4 /* Process abort signal.                           */
	#define SIGBUS     5 /* Access to an undefined portion of a memory obj. */
	#define SIGCHLD    6 /* Child process terminated or stopped.            */
	#define SIGCONT    7 /* Continue executing, if stopped.                 */
	#define SIGFPE     8 /* Erroneous arithmetic operation.                 */
	#define SIGHUP     9 /* Hangup.                                         */
	#define SIGILL    10 /* Illegal instruction.                            */
	#define SIGINT    11 /* Terminal interrupt signal.                      */
	#define SIGPIPE   12 /* Write on a pipe with no one to read it.         */
	#define SIGQUIT   13 /* Terminal quit signal.                           */
	#define SIGSEGV   14 /* Invalid memory reference.                       */
	#define SIGTERM   15 /* Termination signal.                             */
	#define SIGTSTP   16 /* Terminal stop signal.                           */
	#define SIGTTIN   17 /* Background process attempting read.             */
	#define SIGTTOU   18 /* Background process attempting write.            */
	#define SIGALRM   19 /* Alarm clock.                                    */
	#define SIGUSR1   20 /* User-defined signal 1.                          */
	#define SIGUSR2   21 /* User-defined signal 2.                          */
	#define SIGTRAP   22 /* Trace/breakpoint trap.                          */
	
	/* Default signal handler. */
	#define SIG_DFL (sighandler_t)(1)
	
	/* Ignore signal. */
    #define SIG_IGN (sighandler_t)(2)

#ifndef _ASM_FILE_

	typedef void (*sighandler_t)(int);

#endif /* _ASM_FILE_ */

#endif /* SIGNAL_H_ */

/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <signal.h> - Signals library header.
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
	#define SIG_DFL (sighandler_t)(1)
    #define SIG_IGN (sighandler_t)(2)
    
    /* Other constants. */
    #define SIG_ERR (sighandler_t)(0)

#ifndef _ASM_FILE_

	/* Types. */
	typedef void (*sighandler_t)(int);
	
	/* Function prototypes. */
	extern int kill(pid_t pid, int sig);
	extern sighandler_t signal(int sig, sighandler_t func);
	extern int kill(pid_t pid, int sig);

#endif /* _ASM_FILE_ */

#endif /* SIGNAL_H_ */

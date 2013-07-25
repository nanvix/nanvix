/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <signal.h> - Signals library header.
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
	
	/*
	 * DESCRIPTION:
	 *   The kill() function sendS a signal to a process or a group of processes
	 *   specified by pid. The signal to be sent is specified by sig and is either
	 *   one from the list given in <signal.h> or 0. If sig is 0 (the null signal),
	 *   error checking is performed but no signal is actually sent. The null 
	 *   signal can be used to check the validity of pid.
	 * 
	 *   For a process to have permission to send a signal to a process designated
	 *   by pid, unless the sending process has appropriate privileges, the real 
	 *   or effective user ID of the sending process shall match the real or 
	 *   saved set-user-ID of the receiving process. 
	 * 
	 *   The user ID tests described above are not applied when sending SIGCONT 
	 *   to a process that is a member of the same session as the sending process.
	 * 
	 *   If pid is greater than 0, sig is sent to the process whose process ID 
	 *   is equal to pid.
	 * 
	 *   If pid is 0, sig is sent to all processes (excluding an unspecified set
	 *   of system processes) whose process group ID is equal to the process
	 *   group ID of the sender, and for which the process has permission to 
	 *   send a signal.
	 * 
	 *   If pid is -1, sig is sent to all processes (excluding an unspecified
	 *   set of system processes) for which the process has permission to send 
	 *   that signal.
	 * 
	 *   If pid is negative, but not -1, sig is sent to all processes (excluding
	 *   an unspecified set of system processes) whose process group ID is equal
	 *   to the absolute value of pid, and for which the process has permission 
	 *   to send a signal.
	 * 
	 *   If the value of pid causes sig to be generated for the sending process,
	 *   and if sig is not blocked for the calling thread and if no other thread
	 *   has sig unblocked or is waiting in a sigwait() function for sig, either
	 *   sig or at least one pending unblocked signal shall be delivered to the 
	 *   sending thread before kill() returns.
	 * 
	 *   The kill() function is successful if the process has permission to send 
	 *   sig to any of the processes specified by pid. If kill() fails, no signal 
	 *   shall be sent.
	 * 
	 * RETURN VALUE:
	 *   Upon successful completion, 0 shall be returned. Otherwise, -1 shall be 
	 *   returned and errno set to indicate the error.
	 * 
	 * ERRORS:
	 *   [EINVAL] The value of the sig argument is an invalid or unsupported 
	 *            signal number.
	 * 
	 *   [EPERM] The process does not have permission to send the signal to any 
	 *           receiving process.
	 * 
	 *   [ESRCH] No process or process group can be found corresponding to that 
	 *           specified by pid. 
     * 
     * CONFORMING TO:
     *   POSIX.1-2001.
	 */
	extern int kill(pid_t pid, int sig);

#endif /* _ASM_FILE_ */

#endif /* SIGNAL_H_ */

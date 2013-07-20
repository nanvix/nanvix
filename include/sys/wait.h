/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/wait.h> - Declarations for waiting.
 */

#ifndef SYS_WAIT_H_
#define SYS_WAIT_H_

	/*
	 * DESCRIPTION:
	 *   The WEXITSTATUS() macro returns the exit status of a process status 
	 *   value.
	 */
	#define WEXITSTATUS(status) \
		(status & 0xff)         \
    
    /*
     * DESCRIPTION:
     *   The WIFEXITED() macro analyzes a process status value and returns true
     *   if the process exited normally.
     */
    #define WIFEXITED(status) \
		((status >> 8) & 1)   \
    
    /*
     * DESCRIPTION:
     *   The WIFSIGNALED() macro analyzes a process status value and returns true
     *   if the process exited due to uncaught signal.
     */
    #define WIFSIGNALED(status) \
		((status >> 9) & 1)     \

	/*
	 * DESCRIPTION:
	 *   The WTERMSIG() macro returns the signal number that caused the process 
	 *   to terminate.
	 */
    #define WTERMSIG(status)    \
		((status >> 16) & 0xff) \
    
    /*
     * DESCRIPTION:
     *   The wait() function causes the calling process to block until a child 
     *   process terminates. If a child process has already terminated, then 
     *   wait() returns immediately.
     * 
     *   Moreover, if stat_loc is not a NULL pointer, status information 
     *   regarding the process termination is stored in the location pointed to
     *   by stat_loc. This information, aka process status value, may be parsed 
     *   by helper macros defined at <sys/wait.h>.
     * 
     *   The behavior of wait() when the calling process has set SIG_IGN to the 
     *   SIGCHLD signal is explained in the NOTES section.
     * 
     * RETURN VALUE:
     *   Upon successful completion, wait() returns the pid of the terminated
     *   child process. Upon failure, -1 is returned and errno is set to indicate
     *   the error.
     * 
     * ERRORS:
     *   [EINVAL] stat_loc is not NULL and points to a memory location that is 
     *            invalid.  The value of the location pointed to by stat_loc 
     *            remains unchanged.
     * 
     *   [EINTR] wait() has been interrupted by a signal other than SIGCHLD.
     *           The value of the location pointed to by stat_loc remains 
     *           unchanged.
     * 
     *   [ECHILD] The calling process has no child processes. The value of the 
     *            location pointed to by stat_loc remains unchanged.
     * 
     * CONFORMING TO:
     *   POSIX.1-2001.
     * 
     * NOTES:
     *   If the calling process has set SIG_IGN to the SIGCHLD signal, it gets 
     *   blocked until all of its child processed have terminated and then 
     *   returns -1, setting errno to ECHILD.
     * 
     * SEE ALSO:
     *   <sys/wait.h>, <errno.h>, <signal.h>
     */
    extern pid_t wait(int *stat_loc);

#endif /* WAIT_H_ */

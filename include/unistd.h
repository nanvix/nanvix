/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <unistd.h> - Standard symbolic constants and types.
 */

#ifndef UNISTD_H_
#define UNISTD_H_

	/*
	 * DESCRIPTION:
	 *   The alarm() function causes the system to generate a SIGALRM signal for
	 *   the calling process after the number of realtime seconds specified by 
	 *   seconds have elapsed. Processor scheduling delays may prevent the process 
	 *   from handling the signal as soon as it is generated.
	 * 
	 *   If seconds is 0, a pending alarm request, if any, is canceled.
	 * 
	 *   Alarm requests are not stacked; only one SIGALRM generation can be 
	 *   scheduled in this manner. If the SIGALRM signal has not yet been generated, 
	 *   the call results in rescheduling the time at which the SIGALRM signal is 
	 *   generated.
	 * 
	 * RETURN VALUE:
	 *   If there is a previous alarm() request with time remaining, alarm() 
	 *   returns a non-zero value that is the number of seconds until the previous
	 *   request would have generated a SIGALRM signal. Otherwise, alarm() returns 
	 *   zero.
	 * 
	 * ERRORS:
	 *   The alarm() function is always successful, and no return value is 
	 *   reserved to indicate an error.
     * 
     * CONFORMING TO:
     *   POSIX.1-2001.
	 */
	extern unsigned alarm(unsigned seconds);
	
	extern void _exit(int status);
	
	/*
	 * DESCRIPTION:
	 *   The pause() function suspends the calling process until delivery of a 
	 *   signal whose action is either to execute a signal-catching function or 
	 *   to terminate the process.
	 *   
	 *   If the action is to terminate the process, pause() does not return. 
	 * 
	 *   If the action is to execute a signal-catching function, pause() returns 
	 *   after the signal-catching function returns.
	 * 
	 * RETURN VALUE:
	 *   Since pause() suspends process execution indefinitely unless interrupted 
	 *   by a signal, there is no successful completion return value. A value of
	 *   -1 is returned and errno set to indicate the error.
	 * 
	 * ERRORS:
	 *   [EINTR] A signal is caught by the calling process and control is returned
	 *           from the signal-catching function. 
     * 
     * CONFORMING TO:
     *   POSIX.1-2001.
	 */
	extern int pause(void);

#endif /* UNISTD_H_ */

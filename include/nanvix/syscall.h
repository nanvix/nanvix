/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * syscall.h - System call library.
 */

#ifndef NANVIX_SYSCALL_H_
#define NANVIX_SYSCALL_H_

	#include <nanvix/const.h>
	#include <sys/types.h>
	#include <signal.h>
	
	/* Number of system calls. */
	#define NR_SYSCALLS 21
	
	/* System call numbers. */
	#define NR_alarm    0 /* alarm()   */
	#define NR_brk      1 /* brk()     */
	#define NR_fork     2 /* fork()    */
	#define NR_getegid  3 /* getegid() */
	#define NR_geteuid  4 /* geteuid() */
	#define NR_getgid   5 /* getgid()  */
	#define NR_getgrp   6 /* getpgrp() */
	#define NR_getpid   7 /* getpid()  */
	#define NR_getppid  8 /* getppid() */
	#define NR_getuid   9 /* getuid()  */
	#define NR_kill    10 /* kill()    */
	#define NR_nice    11 /* nice()    */
	#define NR_pause   12 /* pause()   */
	#define NR_setegid 13 /* setegid() */
	#define NR_seteuid 14 /* seteuid() */
	#define NR_setgid  15 /* setgid()  */
	#define NR_setpgrp 16 /* setpgrp() */
	#define NR_setuid  17 /* setuid()  */
	#define NR_exit    18 /* exit()    */
	#define NR_wait    19 /* wait()    */
	#define NR_signal  20 /* signal.   */

#ifndef _ASM_FILE_
	
	/*
	 * DESCRIPTION:
	 *   The sys_alarm() function causes the system to generate a SIGALRM
	 *   signal for the calling process after the number of realtime seconds 
	 *   specified by seconds have been elapsed. Processor scheduling delays may
	 *   prevent the process from handling the signal as soon as it is generated.
	 * 
	 *   If seconds is 0, a pending alarm request, if any, is canceled.
	 * 
	 *   Alarm requests are not stacked; only one SIGALRM generation can be
	 *   scheduled in this manner. If the SIGALRM signal has not yet been 
	 *   generated, the call results in rescheduling the time at which the 
	 *   SIGALRM signal is generated.
	 * 
	 * RETURN VALUE:
	 *   If there is a previous alarm request with time remaining, sys_alarm()
	 *   returns a non-zero value that is the number of seconds until the previous
	 *   request would have generated the SIGALRM signal. Otherwise, sys_alarm() 
	 *   returns 0.
	 * 
	 * ERRORS:
	 *   The sys_alarm() function is always successful and no return value is 
	 *   reserved to indicate an error.
	 */
	EXTERN unsigned sys_alarm(unsigned seconds);
	
	/*
	 * TODO.
	 */
	EXTERN int sys_brk(void *addr);
	
	/*
	 * TODO.
	 */
	EXTERN void sys_exit(int status);
	
	/*
	 * TODO.
	 */
	EXTERN pid_t sys_fork();
	
	/*
	 * DESCRIPTION:
	 *   The sys_getegid() function returns the effective user group ID of the 
	 *   calling process.
	 * 
	 * RETURN VALUE:
	 *   The sys_getegid() function is always successful and no return value is 
	 *   reserved to indicate an error.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN gid_t sys_getegid();
	
	/*
	 * DESCRIPTION:
	 *   The sys_geteuid() function returns the effective user ID of the calling
	 *   process.
	 * 
	 * RETURN VALUE:
	 *   The sys_geteuid() function is always successful and no return value is 
	 *   reserved to indicate an error.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN pid_t sys_geteuid();
	
	/*
	 * DESCRIPTION:
	 *   The sys_getgid() function returns the real group ID of the calling process.
	 * 
	 * RETURN VALUE:
	 *   The sys_getgid() function is always successful and no return value is 
	 *   reserved to indicate an error.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN gid_t sys_getgid();
	
	/*
	 * DESCRIPTION:
	 *   The sys_getpgrp() function returns the process group ID of the calling 
	 *   process.
	 * 
	 * RETURN VALUE:
	 *   The sys_getpgrp() function is always successful and no return value is 
	 *   reserved to indicate an error.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN pid_t sys_getpgrp();
	
	/*
	 * DESCRIPTION:
	 *   The sys_getpid() function gets the process ID of the calling process.
	 * 
	 * RETURN VALUE:
	 *   The sys_getpid() function returns the process ID of the calling process.
	 *   
	 *   The sys_getpid() function is always successful and no return value is 
	 *   reserved to indicate an error.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN pid_t sys_getpid();
	
	/*
	 * DESCRIPTION:
	 *   The sys_getppid() function returns the parent process ID of the calling 
	 *   process.
	 * 
	 * RETURN VALUE:
	 *   The sys_getppid() function is always successful and no return value is 
	 *   reserved to indicate an error.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN pid_t sys_getppid();
	
	/*
	 * DESCRIPTION:
	 *   The sys_getuid() function gets the real user ID of the calling process.
	 * 
	 * RETURN VALUE:
	 *   The sys_getuid() function returns the real user ID of the calling process.
	 *   
	 *   The sys_getuid() function is always successful and no return value is 
	 *   reserved to indicate an error.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN pid_t sys_getuid();
	
	/*
	 * DESCRIPTION:
	 *   The sys_kill() function attempts to send a signal to a process or a 
	 *   group of processes specified by pid. The signal to be sent is sepcified
	 *   by sig is either one from the list given in <signal.h>. If sig is the 
	 *   SIGNULL signal, error checking is performed but no signal is actually
	 *   sent. The null signal can be used to check the validity of pid.
	 * 
	 *   For a process to have permission to send a signal to a process designated
	 *   by pid, unless the sending process has appropriate privileges, the real
	 *   or effective user ID of the sending process shall match the real or saved
	 *   set-user-ID of the receiving process.
	 * 
	 *   If pid is greater than 0, sig shall be sent to the process whose process
	 *   ID is equal to pid.
	 * 
	 *   If pid is 0, sig shall be sent to the process whose process ID is equal
	 *   to pid.
	 * 
	 *   If pid is 0, sig shall be sent to all processes (exclusing an unspecified
	 *   set of system processes) whose process group ID is equal to the process
	 *   group ID of the sender, and for which the process has permission to send
	 *   a signal.
	 * 
	 *   If pid is -1, sig shall be sent to all processes (excluding an unspecified
	 *   set of system processes) for which the process has permission to send 
	 *   that signal.
	 *  
	 *   If pid is negative, but not -1, sig shall be sent to all processes 
	 *   (excluding an unspecified set of system processes) whose process group 
	 *   ID is equal to the absolute value of pid, and for which the process has
	 *   permission to send a signal.
	 *   
	 *   The user ID tests described above shall not be applied when sending 
	 *   SIGCONT to a process that is a member of the same session as the sending
	 *   process.
	 * 
	 *   The sys_kill() function is successful if the process has permission to 
	 *   send sig to any of the processes specified by pid. If sys_kill() fails,
	 *   no signal is sent.
	 * 
	 * RETURN VALUE:
	 * 	 Upon successful completion, sys_kill() returns zero. Upon failure, a 
	 *   negative error code is returned instead.
	 * 
	 * ERRORS:
	 *   - [EINVAL] The value of the sig argument is an invalid or unsupported 
	 *              signal number.
	 * 
	 *   - [EPERM] The process does not have permission to send the signal to 
	 *             any receiving process.
	 *
	 *   - [ESRCH] No process or process group can be found corresponding to that
	 *             specified by pid.
	 * 
	 * SEE ALSO:
	 *   <signal.h>
	 */
	EXTERN int sys_kill(pid_t pid, int sig);
	
	/*
	 * DESCRIPTION:
	 *   The sys_nice() function shall add the value of incr to the nice value 
	 *   of the calling process. A process' nice value is a non-negative number 
	 *   for which a more positive value shall result in less favorable 
	 *   scheduling.
	 * 
	 *   A maximum nice value of 2*{NZERO}-1 and a minimum nice value of 0 are 
	 *   imposed by the system. Requests for values above or below these limits 
	 *   result in the nice value being set to the corresponding limit. Only a 
	 *   process with appropriate privileges can lower the nice value.
	 * 
	 * RETURN VALUE:
	 *   Upon successful completion, sys_nice() returns the new nice value. Upon
	 *   failure, a negative error code is returned instead.
	 * 
	 * ERRORS:
	 *   - [EPERM] The incr argument is negative and the calling process does
	 *             not have appropriate privileges.
	 */
	EXTERN int sys_nice(int incr);
	
	/*
	 * DESCRIPTION:
	 *   The sys_pause() function suspends the calling process until the delivery
	 *   of a signal whose action is either to execute a signal-catching function
	 *   or to terminate the process.
	 *
	 * RETURN VALUE:
	 *   The sys_pause() function has no successful completion since it suspends
	 *   the process execution indefinitely unless interrupted by a signal.
	 *   Therefore, sys_pause() returns a negative error code.
	 * 
	 * ERRORS:
	 *   - [EINTR] A signal whose action is either to execute a signal-catching 
	 *             function or to terminate has been delivered to the process.
	 */
	EXTERN void sys_pause();
	
	/*
	 * DESCRIPTION:
	 *   The sys_setegid() function sets the effective user group ID of the calling 
	 *   process. 
	 * 
	 *   If gid is equal to the real group ID or the saved set-group-ID, or if 
	 *   the process has appropriate privileges, sys_setegid() sets the effective
	 *   group ID of the calling process to gid; the real group ID, saved 
	 *   set-group-ID, and any supplementary group IDs shall remain unchanged.
	 * 
	 *   The sys_setegid() function does not affect the supplementary group list
	 *   in any way.
	 * 
	 * RETURN VALUE:
	 *   Upon successful completion, the sys_setegid() function returns zero.
	 *   Upon failure, a negative error code is returned instead.
	 * 
	 * ERRORS:
	 *   - [EPERM] The process does not have appropriate privileges and gid does
	 *             not match the real group ID or the saved set-group-ID.
	 */
	EXTERN int sys_setegid(gid_t gid);
	
	/*
	 * DESCRIPTION:
	 *   The sys_seteuid() function sets the effective user ID of the calling 
	 *   process. 
	 * 
	 *   If uid is equal to the real user ID or the saved set-user-ID, or if the
	 *   process has appropriate privileges, sys_setuid() sets the effective user
	 *   ID of the calling process to uid; the real user ID and saved 
	 *   set-user-ID shall remain unchanged.
	 * 
	 *   The sys_seteuid() function does not affect the supplementary group list
	 *   in any way.
	 * 
	 * RETURN VALUE:
	 *   Upon successful completion, the sys_seteuid() function returns zero.
	 *   Upon failure, a negative error code is returned instead.
	 * 
	 * ERRORS:
	 *   - [EPERM] The process does not have appropriate privileges and uid does
	 *             not match the real user ID or the saved set-user-ID. 
	 */
	EXTERN int sys_seteuid(uid_t uid);

	/*
	 * DESCRIPTION:
	 *   The sys_setgid() function sets the group ID of the calling process. 
	 *
	 *   If the process has appropriate privileges, sys_setgid() shall set the 
	 *   real group ID, effective group ID, and the saved set-group-ID of the 
	 *   calling process to gid. 
	 *
	 *   If the process does not have appropriate privileges, but gid is equal 
	 *   to the real group ID or the saved set-group-ID, sys_setgid() shall set 
	 *   the effective user ID to gid; the real group ID and saved set-group-ID 
	 *   shall remain unchanged.
	 * 
	 *   The sys_setgid() function does not affect the supplementary group list
	 *   in any way.
	 * 
	 * RETURN VALUE:
	 *   Upon successful completion, the sys_setgid() function returns zero.
	 *   Upon failure, a negative error code is returned instead.
	 *
	 * ERRORS:
	 *   - [EPERM] The process does not have appropriate privileges and gid does 
	 *             not match the real group ID or the saved set-group-ID.
	 */
	EXTERN int sys_setgid(gid_t uid);
	
	EXTERN sighandler_t sys_signal(int signum, sighandler_t handler, void (*restorer)(void));
	
	/*
	 * DESCRIPTION:
	 *   If the calling process is not already a session leader, sys_setpgrp() 
	 *   sets the process group ID of the calling process to the process ID of 
	 *   the calling process. If sys_setpgrp() creates a new session, then the 
	 *   new session has no controlling terminal.
	 * 
	 *   The sys_setpgrp() function has no effect when the calling process is a 
	 *   session leader.
	 * 
	 * RETURN VALUE:
	 *   Upon completion, sys_setpgrp() returns the process group ID.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN pid_t sys_setpgrp();
	
	/*
	 * DESCRIPTION:
	 *   The sys_setuid() function sets the user ID of the calling process. 
	 *
	 *   If the process has appropriate privileges, sys_setuid() sets the real 
	 *   user ID, effective user ID, and the saved set-user-ID of the calling 
	 *   process to uid. 
	 *
	 *   If the process does not have appropriate privileges, but uid is equal 
	 *   to the real user ID or the saved set-user-ID, sys_setuid() shall set the 
	 *   effective user ID to uid; the real user ID and saved set-user-ID shall 
	 *   remain unchanged.
	 * 
	 *   The sys_setuid() function does not affect the supplementary group list
	 *   in any way.
	 * 
	 * RETURN VALUE:
	 *   Upon successful completion, the sys_setuid() function returns zero.
	 *   Upon failure a negative error code is returned instead.
	 *
	 * ERRORS:
	 *   - [EPERM] The process does not have appropriate privileges and uid does 
	 *             not match the real user ID or the saved set-user-ID.
	 */
	EXTERN int sys_setuid(pid_t uid);
	
	/*
	 * DESCRIPTION:
	 *   The sys_wait() function suspends execution of the calling process until
	 *   a child process terminates. Moreover, iIf stat_loc is not a null pointer, 
	 *   information regarding the status of the terminated child process is stored 
	 *   in the location pointed to by stat_loc.
	 * 
	 * RETURN VALUE:
	 *   Upon successful completion, sys_wait() returns the process ID of the
	 *   terminated child process. Upon failure a negative error code is returned
	 *   instead.
	 * 
	 * ERRORS:
	 *   - [ECHILD] The calling process has no existing unwaited-for child processes
	 *              or the calling process has SIGCHLD set to SIG_IGN.
	 * 
	 *   - [EINTR] The function was interrupted by a signal.
	 * 
	 *   - [EPERM] The process has no permissions to write to the location pointed
	 *             to by stat_loc. 
	 */
	EXTERN pid_t sys_wait(int *stat_loc);

#endif /* _ASM_FILE_ */

#endif /* NANVIX_SYSCALL_H_ */

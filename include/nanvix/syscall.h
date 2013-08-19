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
	#define NR_alarm    0
	#define NR_brk      1
	#define NR_fork     2
	#define NR_getegid  3
	#define NR_geteuid  4
	#define NR_getgid   5
	#define NR_getgrp   6
	#define NR_getpid   7
	#define NR_getppid  8
	#define NR_getuid   9
	#define NR_kill    10
	#define NR_nice    11
	#define NR_pause   12
	#define NR_setegid 13
	#define NR_seteuid 14
	#define NR_setgid  15
	#define NR_setpgrp 16
	#define NR_setuid  17
	#define NR__exit   18
	#define NR_wait    19
	#define NR_signal  20

#ifndef _ASM_FILE_

	/* System calls prototypes. */
	EXTERN unsigned sys_alarm(unsigned seconds);
	EXTERN int sys_brk(void *addr);
	EXTERN void sys__exit(int status);
	EXTERN pid_t sys_fork(void);
	EXTERN pid_t sys_getpgrp(void);
	EXTERN pid_t sys_getpid(void);
	EXTERN pid_t sys_getppid(void);
	EXTERN pid_t sys_getuid(void);
	EXTERN int sys_kill(pid_t pid, int sig);
	EXTERN int sys_nice(int incr);
	EXTERN void sys_pause(void);
	EXTERN int sys_setegid(gid_t gid);
	EXTERN int sys_seteuid(uid_t uid);
	EXTERN int sys_setgid(gid_t uid);	
	EXTERN sighandler_t sys_signal(int sig, sighandler_t func);
	EXTERN pid_t sys_setpgrp(void);
	EXTERN int sys_setuid(pid_t uid);
	EXTERN pid_t sys_wait(int *stat_loc);
	
	/*
	 * Gets the effective user group ID of the calling process.
	 */
	EXTERN gid_t sys_getegid(void);
	
	/*
	 *  Gets the effective user ID of the calling process.
	 */
	EXTERN uid_t sys_geteuid(void);
	
	/*
	 * Gets the real user group ID of the calling process.
	 */
	EXTERN gid_t sys_getgid(void);

#endif /* _ASM_FILE_ */

#endif /* NANVIX_SYSCALL_H_ */

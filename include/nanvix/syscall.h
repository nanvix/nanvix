/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * syscall.h - System call library.
 */

#ifndef NANVIX_SYSCALL_H_
#define NANVIX_SYSCALL_H_

	#include <nanvix/const.h>
	#include <sys/stat.h>
	#include <sys/types.h>
	#include <signal.h>
	
	/* Number of system calls. */
	#define NR_SYSCALLS 37
	
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
	#define NR_access  21
	#define NR_chdir   22
	#define NR_chown   23
	#define NR_chroot  24
	#define NR_chmod   25
	#define NR_open    26
	#define NR_umask   27
	#define NR_read    28
	#define NR_write   29
	#define NR_close   30
	#define NR_execve  31
	#define NR_lseek   32
	#define NR_pipe    33
	#define NR_stat    34
	#define NR_fcntl   35
	#define NR_sync    36
	

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
	EXTERN int sys_seteuid(uid_t uid);
	EXTERN int sys_setgid(gid_t uid);	
	EXTERN sighandler_t sys_signal(int sig, sighandler_t func);
	EXTERN pid_t sys_setpgrp(void);
	EXTERN int sys_setuid(pid_t uid);
	EXTERN pid_t sys_wait(int *stat_loc);
	
	/*
	 * Checks user permissions for a file.
	 */
	EXTERN int sys_access(const char *path, int amode);
	
	/*
	 * Changes working directory.
	 */
	EXTERN int sys_chdir(const char *path);
	
	/*
	 * Changes permissions of a file.
	 */
	EXTERN int sys_chmod(const char *path, mode_t mode);
	
	/*
	 * Changes owner and group of a file.
	 */
	EXTERN int sys_chown(const char *path, uid_t owner, gid_t group);
	
	/*
	 * Changes working directory.
	 */
	EXTERN int sys_chroot(const char *path);
	
	/*
	 * Closes a file.
	 */
	EXTERN int sys_close(int fd);
	
	/*
	 * Executes a program.
	 */
	EXTERN int sys_execve(const char *filename, const char **argv, const char **envp);
	
	/*
	 * Manipulates file descriptor.
	 */
	EXTERN int sys_fcntl(int fd, int cmd, int arg);
	
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
	
	/*
	 * Gets the parent process ID of the calling process.
	 */
	EXTERN pid_t sys_getppid(void);
	
	/*
	 * Gets the real user ID of the calling process.
	 */
	EXTERN uid_t sys_getuid(void);
	
	/*
	 * Moves the read/write file offset.
	 */
	EXTERN off_t sys_lseek(int fd, off_t offset, int whence);
	
	/*
	 * Opens a file.
	 */
	EXTERN int sys_open(const char *path, int oflag, mode_t mode);
	
	/*
	 * Creates an interprocess channel.
	 */
	EXTERN int sys_pipe(int fildes[2]);
	
	/*
	 * Reads from a file.
	 */
	EXTERN ssize_t sys_read(int fd, void *buf, size_t n);
	
	/*
	 * Sets the effective user group ID of the calling process.
	 */
	EXTERN int sys_setegid(gid_t gid);
	
	/*
	 * Sets the real user group ID of the calling process.
	 */
	EXTERN int sys_setgid(gid_t gid);
	
	/*
	 * Sets the effective user ID of the calling process.
	 */
	EXTERN int sys_seteuid(uid_t uid);
	
	/*
	 * Sets the real user ID of the calling process.
	 */
	EXTERN int sys_setuid(pid_t uid);
	
	/*
	 * Gets file status.
	 */
	EXTERN int sys_stat(const char *path, struct stat *buf);
	
	/*
	 * Schedules file system updates.
	 */
	EXTERN void sys_sync(void);
	
	/*
	 * Sets and gets the file mode creation mask.
	 */
	EXTERN mode_t sys_umask(mode_t cmask);
	
	/*
	 * Writes to a file.
	 */
	EXTERN ssize_t sys_write(int fd, const void *buf, size_t n);

#endif /* _ASM_FILE_ */

#endif /* NANVIX_SYSCALL_H_ */

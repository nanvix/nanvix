/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <unistd.h> - Standard symbolic constants and types.
 */

#ifndef UNISTD_H_
#define UNISTD_H_

	#include <sys/stat.h>
	#include <sys/types.h>
	#include <fcntl.h>

	/* Tests for access(). */
	#define F_OK 0 /* File exists. */
	#define R_OK 1 /* May read.    */
	#define W_OK 2 /* May write.   */
	#define X_OK 4 /* May execute. */
	
	/* Starting positions for lseek() and fcntl(). */
	#define SEEK_CUR 0 /* Set file offset to current plus offset. */
	#define SEEK_END 1 /* Set file offset to EOF plus offset.     */
	#define SEEK_SET 2 /* Set file offset to offset.              */

	extern unsigned alarm(unsigned seconds);
	extern void _exit(int status);
	extern pid_t getpid(void);
	extern pid_t getpgrp(void);
	extern int pause(void);
	
	/*
	 * Checks user permissions for a file.
	 */
	extern int access(const char *path, int amode);
	
	/*
	 * Changes working directory.
	 */
	extern int chdir(const char *path);
	
	/*
	 * Changes owner and group of a file.
	 */
	extern int chown(const char *path, uid_t owner, gid_t group);
	
	/*
	 * Changes working directory.
	 */
	extern int chroot(const char *path);
	
	/*
	 * Closes a file.
	 */
	extern int close(int fd);
	
	/*
	 * Duplicates an opened file descriptor.
	 */
	#define dup(fd) \
		fcntl(fd, F_DUPFD, 0)
	
	/*
	 * Executes a program.
	 */
	extern int execve(const char *filename, const char **argv, const char **envp);

	/*
	 * Gets the effective user group ID of the calling process.
	 */
	extern gid_t getegid(void);
	
	/*
	 *  Gets the effective user ID of the calling process.
	 */
	extern uid_t geteuid(void);
	
	/*
	 * Gets the real user group ID of the calling process.
	 */
	extern gid_t getgid(void);
	
	/*
	 * Gets the parent process ID of the calling process.
	 */
	extern pid_t getppid(void);
	
	/*
	 * Moves the read/write file offset.
	 */
	extern off_t lseek(int fd, off_t offset, int whence);
	
	/*
	 * Gets the real user ID of the calling process.
	 */
	extern uid_t getuid(void);
	
	/*
	 * Creates an interprocess channel.
	 */
	extern int pipe(int fildes[2]);
	
	/*
	 * Reads from a file.
	 */
	extern ssize_t read(int fd, void *buf, size_t n);
	
	/*
	 * Sets the effective user group ID of the calling process.
	 */
	extern int setegid(gid_t gid);
	
	/*
	 * Sets the real user group ID of the calling process.
	 */
	extern int setgid(gid_t gid);
	
	/*
	 * Sets the effective user ID of the calling process.
	 */
	extern int seteuid(uid_t uid);
	
	/*
	 * Sets the real user ID of the calling process.
	 */
	extern int setuid(pid_t uid);
	
	/*
	 * Writes to a file.
	 */
	extern ssize_t write(int fd, const void *buf, size_t n);

#endif /* UNISTD_H_ */

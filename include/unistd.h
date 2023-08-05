/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2016 Davidson Francis <davidsondfgl@hotmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
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
	 * Changes process' breakpoint value.
	 */
	extern void *sbrk(size_t size);

	/*
	 * Changes process' breakpoint value.
	 */
	extern int brk(void *ptr);

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
	 * Duplicates a file descriptor.
	 */
	extern int dup2(int oldfd, int newfd);

	/*
	 * Executes a program.
	 */
	#define execv(path, argv) \
		execve(path, argv, (char *const*)environ);

	/*
	 * Executes a program.
	 */
	extern int execve(const char *filename, char *const argv[],  char *const envp[]);

	/*
	 * Executes a program.
	 */
	extern int execvp(const char *file, char *const argv[]);

	/*
	 * Creates a new process.
	 */
	extern pid_t fork(void);

	/*
	 * Gets the pathname of the current working directory.
	 */
	extern char *getcwd(char *buf, size_t size);

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
	 * Links a name to a file.
	 */
	extern int link(const char *path1, const char *path2);

	/*
	 * Moves the read/write file offset.
	 */
	extern off_t lseek(int fd, off_t offset, int whence);

	/*
	 * Changes the nice value of the calling process.
	 */
	extern int nice(int incr);

	/*
	 * Gets the real user ID of the calling process.
	 */
	extern uid_t getuid(void);

	/*
	 * Tests whether a file descriptor refers to a terminal.
	 */
	extern int isatty(int fd);

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

	extern pid_t setpgrp(void);

#ifndef __NANVIX_KERNEL__

    /*
     * Puts the current process to sleep.
     */
    extern unsigned sleep(unsigned seconds);

#endif /* __NANVIX_KERNEL__ */

	/*
	 * Schedules file system updates.
	 */
	extern void sync(void);

	/*
	 * Removes a directory entry.
	 */
	extern int unlink(const char *path);

	/*
	 * Writes to a file.
	 */
	extern ssize_t write(int fd, const void *buf, size_t n);

	extern int shutdown(void);

	/*
	 * Gets process information
	 */
	extern int ps(void);

	/*
	 * Get system ticks
	 */
	extern int gticks(void);

	/* Environment variables. */
	extern char **environ;

#endif /* UNISTD_H_ */

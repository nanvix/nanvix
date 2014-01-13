/*
 * Copyright(C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * <errno.h> - System error codes
 */

#ifndef ERRNO_H_
#define ERRNO_H_
	
	/* Resource unavailable, try again. */
	#define EAGAIN 1
	
	/* Device or resource busy. */
	#define EBUSY 2
	
	/* No child processes. */
	#define ECHILD 3
	
	/* Interrupted function. */
	#define EINTR 4
	
	/* Invalid argument. */
	#define EINVAL 5
	
	/* Not enough space. */
	#define ENOMEM 6
	
	/* Function not supported. */
	#define ENOSYS 7
	
	/* Operation not permitted. */
	#define EPERM 8
	
	/* No such process. */
	#define ESRCH 9
	
	/* Not supported. */
	#define ENOTSUP 10
	
	/* Filename too long. */
	#define ENAMETOOLONG 11
	
	/* Not a directory. */
	#define ENOTDIR 12
	
	/* No such file or directory. */
	#define ENOENT 13
	
	/* Permission denied. */
	#define EACCES 14
	
	/* Too many open files. */
	#define EMFILE 15
	
	/* Too many files open in system. */
	#define ENFILE 16
	
	/* File exists. */
	#define EEXIST 17
	
	/* No space left on device. */
	#define ENOSPC 18
	
	/* Is a directory. */
	#define EISDIR 19
	
	/* Bad file descriptor. */
	#define EBADF 20
	
	/* File too large. */
	#define EFBIG 21
	
	/* Executable file format error. */
	#define ENOEXEC 22
	
	/* Argument list too long. */
	#define E2BIG 23
	
	/* Bad address. */
	#define EFAULT  24
	
	/* Invalid seek. */
	#define ESPIPE 25
	
	/* Broken pipe. */
	#define EPIPE 26

#ifndef _ASM_FILE_

	/* Number of last error. */
	extern int errno;

#endif /* _ASM_FILE_ */

#endif /* ERRNO_H_ */

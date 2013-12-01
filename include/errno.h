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

#endif /* ERRNO_H_ */

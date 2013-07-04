/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * errno.h - System error codes
 */

#ifndef ERRNO_H_
#define ERRNO_H_
	
	/* Resource unavailable, try again. */
	#define EAGAIN 1
	
	/* Device or resource busy. */
	#define EBUSY 2
	
	/* Interrupted function. */
	#define EINTR 3
	
	/* Invalid argument. */
	#define EINVAL 4
	
	/* Not enough space. */
	#define ENOMEM 5
	
	/* Operation not permitted. */
	#define EPERM 6
	
	/* No such process. */
	#define ESRCH 7

#endif /* ERRNO_H_ */

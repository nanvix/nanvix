/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * limits.h - System limits.
 */

#ifndef LIMITS_H_
#define LIMITS_H_

	/* Number of functions that may be registered with atexit() */
	#define ATEXIT_MAX 32

	/* Default process priority. */
	#define NZERO 20
	
	/* Bytes in a filename. */
	#define NAME_MAX 14
	
	/* Files that one process can have open simultaneously. */
	#define OPEN_MAX 20
	
	/* Length of argument to the execve(). */
	#define ARG_MAX 2048
	
	/* Number of bytes in pathname. */
	#define PATH_MAX 512

#endif /* LIMITS_H_ */

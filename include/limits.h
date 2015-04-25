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
	
	/* Maximum number of links to a single file. */
	#define LINK_MAX 8
	
	/* Maximum value of a long. */
	#define LONG_MAX 2147483647
	
	/* Minimum value of type long. */
	#define LONG_MIN -2147483647
	
	/* Maximum value for an object of type int. */
	#define INT_MAX 2147483647
	
	/* Minimum value for an object of type int. */
	#define INT_MIN -2147483647
	
	/**
	 * @brief Maximum value for an object of type long long.
	 */
	#define LLONG_MAX +9223372036854775807
	
	/**
	 * @brief Minimum value for an object of type long long.
	 */
	#define LLONG_MIN -9223372036854775807
	
	/* Bytes in a filename. */
	#define NAME_MAX 14
	
	/* Files that one process can have open simultaneously. */
	#define OPEN_MAX 20
	
	/* Length of argument to the execve(). */
	#define ARG_MAX 2048
	
	/* Number of bytes in pathname. */
	#define PATH_MAX 512
	
	/* Maximum value for unsigned long. */
	#define ULONG_MAX 4294967295u

#endif /* LIMITS_H_ */

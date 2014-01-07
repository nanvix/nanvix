/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * limits.h - System limits.
 */

#ifndef LIMITS_H_
#define LIMITS_H_

	/* Default process priority. */
	#define NZERO 20
	
	/* Maximum number of bytes in a filename. */
	#define NAME_MAX 14
	
	/* Maximum number of files that one process can have open simultaneously. */
	#define OPEN_MAX 20
	
	/* Maximum length of argument to the execve(). */
	#define ARG_MAX 2048

#endif /* LIMITS_H_ */


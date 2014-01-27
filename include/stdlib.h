/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <stdlib.h> - Standard library.
 */

#ifndef STDLIB_H_
#define STDLIB_H_

	#include <sys/types.h>

	/* Unsuccessful termination. */
	#define EXIT_FAILURE 1
	
	/* Successful termination. */
	#define EXIT_SUCCESS 0

	/* Null pointer. */
	#ifndef NULL
		#define NULL ((void *)0)
	#endif
	
	/*
	 * Terminates the calling process.
	 */
	extern void exit(int status);
	
	/*
	 * Frees memory.
	 */
	extern void free(void *ptr);
	
	/*
	 * Gets value of an environment variable.
	 */
	extern char *getenv(const char *name);
	
	/*
	 * Allocates memory.
	 */
	extern void *malloc(size_t size);

#endif /* STDLIB_H_ */

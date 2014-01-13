/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <stdlib.h> - Standard library.
 */

#ifndef STDLIB_H_
#define STDLIB_H_

	#include <sys/types.h>

	/* Null pointer. */
	#ifndef NULL
		#define NULL ((void *)0)
	#endif
	
	/*
	 * Allocates memory.
	 */
	extern void *malloc(size_t size);

#endif /* STDLIB_H_ */

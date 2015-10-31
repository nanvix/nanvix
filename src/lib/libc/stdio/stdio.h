/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/stdio.h - Private standard IO library.
 */

#ifndef _STDIO_H_
#define _STDIO_H_

	#include <stdio.h>
	
	/* Forward definitions. */
	extern FILE *_getstream(void);
	extern int _sflags(const char *, int *);
	
	/* File streams table. */
	extern FILE streams[FOPEN_MAX];

#endif /* _STDIO_H_ */

/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/stdio.h - Private standard IO library.
 */

#ifndef _STDIO_H_
#define _STDIO_H_

	#include <stdio.h>
	
	/* File streams table. */
	extern FILE streams[FOPEN_MAX];

#endif /* _STDIO_H_ */

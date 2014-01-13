/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/stdio.c - Internal stdio library stuff.
 */

#include <fcntl.h>
#include <stdlib.h>
#include <stdio.h>

/* File streams table. */
FILE streams[FOPEN_MAX] = {
	{ 0, 0, 0, 0, 0, O_RDONLY, NULL, 0, 0, 0, 0, 0 },
	{ 1, 0, 0, 0, 0, O_WRONLY, NULL, 0, 0, 0, 0, 0 },
	{ 2, 0, 0, 0, 0, O_WRONLY, NULL, 0, 0, 0, 0, 0 },
};

/* Standard file streams. */
FILE *stdin = &streams[0];  /* Standard input.  */
FILE *stdout = &streams[1]; /* Standard output. */
FILE *stderr = &streams[2]; /* Standard error.  */

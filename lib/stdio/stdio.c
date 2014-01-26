/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/stdio.c - Internal stdio library stuff.
 */

#include <stdlib.h>
#include <stdio.h>

/* File streams table. */
FILE streams[FOPEN_MAX] = {
	{ 0, _IOREAD  | _IOLBF, NULL, NULL, 0, 0 },
	{ 1, _IOWRITE | _IOFBF, NULL, NULL, 0, 0 },
	{ 2, _IOWRITE | _IONBF, NULL, NULL, 0, 0 },
};

/* Standard file streams. */
FILE *stdin = &streams[0];  /* Standard input.  */
FILE *stdout = &streams[1]; /* Standard output. */
FILE *stderr = &streams[2]; /* Standard error.  */

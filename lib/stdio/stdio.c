/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/stdio.c - Internal stdio library stuff.
 */

#include <stdlib.h>
#include <stdio.h>

/* File streams table. */
FILE streams[FOPEN_MAX] = {
	{ 0, _IOREADING | _IOREAD  | _IOLBF, NULL, NULL, 0, -1, -1 },
	{ 1, _IOWRITING | _IOWRITE | _IOFBF, NULL, NULL, 0, -1, -1 },
	{ 2, _IOWRITING | _IOWRITE | _IONBF, NULL, NULL, 0, -1, -1 },
};

/* Standard file streams. */
FILE *stdin = &streams[0];  /* Standard input.  */
FILE *stdout = &streams[1]; /* Standard output. */
FILE *stderr = &streams[2]; /* Standard error.  */

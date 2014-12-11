/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/fputs.c - fputs() implementation.
 */

#include <stdio.h>
#include <string.h>

/*
 * Writes a string to a file.
 */
int fputs(const char *str, FILE *stream)
{
	int c;       /* Working character. */
	int ret = 0; /* Return value.      */
	
	/* Write string. */
	while ((c = *str++) != '\0')
		ret = putc(c, stream);
		
	return (ret);
}

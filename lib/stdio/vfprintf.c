/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/vfprintf.c - vfprintf() implementation.
 */

#include <stdio.h>
#include <stdarg.h>

/*
 * Writes format output of a stdarg argument list to a file.
 */
int vfprintf(FILE *stream, const char *format, va_list ap)
{
	int n;             /* Characters written. */
	char buffer[1024]; /* Buffer.             */
	
	/* Format string. */
	n = vsprintf(buffer, format, ap);
	
	/* Write formated string to file. */
	if (n > 0)
		fputs(buffer, stream);
	
	return (n);
}


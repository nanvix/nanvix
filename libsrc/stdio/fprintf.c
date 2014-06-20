/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/fprintf.c - fprintf() implementation.
 */

#include <stdio.h>
#include <stdarg.h>

/*
 * Writes a formated string to a file.
 */
int fprintf(FILE *stream, const char *format, ...)
{
	int n;        /* Characters written. */
	va_list args; /* Arguments.          */
	
	va_start(args, format);
	n = vfprintf(stream, format, args);
	va_end(args);
	
	return (n);
}

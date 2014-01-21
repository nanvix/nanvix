/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepena@gmail.com>
 * 
 * stdio/printf.c - printf() implementation.
 */

#include <stdio.h>
#include <stdarg.h>

/*
 * Writes a formated string to the standard output file.
 */
int printf(const char *format, ...)
{
	int n;        /* Characters written. */
	va_list args; /* Arguments.          */
	
	va_start(args, format);
	n = vfprintf(stdout, format, args);
	va_end(args);
	
	return (n);
}

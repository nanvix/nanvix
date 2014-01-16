/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/fputc.c - fputc() implementation.
 */
#include <stdio.h>

/*
 * Writes a character to a file.
 */
int fputc(int c, FILE *stream)
{
	return (putc(c, stream));
}

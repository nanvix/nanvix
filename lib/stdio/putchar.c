/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/putchar.c - putchar() implementation.
 */

#include <stdio.h>
#include "stdio.h"

/*
 * Writes a character to the standard output file.
 */
int putchar(int c)
{
	return (putc(c, stdout));
}

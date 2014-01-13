/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/puts.c - puts() implementation.
 */

#include <stdio.h>
#include <string.h>

/*
 * Writes a string to the standard output file.
 */
int puts(const char *str)
{
	int length;
	
	length = strlen(str);
	
	((char *)str)[length++] = '\n';
	
	return (fwrite(str, length*sizeof(char), 1, stdout));
}

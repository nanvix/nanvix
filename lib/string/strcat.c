/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * string/strcat.c - strcat() implementation.
 */

#include <sys/types.h>
#include <string.h>

/*
 * Concatenates two strings.
 */
char *strcat(char *dest, const char *src)
{
	strcpy(dest + strlen(dest), src);	
	return (dest);
}

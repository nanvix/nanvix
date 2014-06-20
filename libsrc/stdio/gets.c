/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/gets.c - gets() implementation.
 */

#include <stdio.h>

/*
 * Reads a string from the standard input file.
 */
char *gets(char *str)
{
	int c;   /* Working character. */
	char *p; /* Write pointer.     */
	
	p = str;
	
	/* Read string. */
	while ((c = getchar()) != '\n')
		*p++ = c;
	
	*p = '\0';
	
	return (str);
}

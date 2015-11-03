/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/fgets.c - fgets() implementation.
 */

#include <errno.h>
#include <stdlib.h>
#include <stdio.h>

/*
 * Reads a string from a file.
 */
char *fgets(char *str, int n, FILE *stream)
{
	int c = EOF;   /* Working character. */
	char *p = str; /* Write pointer.     */
	
	/* Read string. */
	while ((--n > 0) && ((c = getc(stream)) != EOF))
	{
		if (c == '\n')
			break;
		*p++ = c;
	}
	
	if ((c == EOF) && (p == str))
		return (NULL);
	
	*p = '\0';
	
	return (str);
}

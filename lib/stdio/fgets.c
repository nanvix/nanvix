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
	int c;   /* Working character. */
	char *p; /* Write pointer.     */
	
	p = str;
	
	/* Read string. */
	while ((--n > 0) && ((c = getc(stream)) != EOF))
	{
		*p++ = c;
		
		if (c == '\n')
			break;
			
	}
	
	if ((c == EOF) && (p == str))
		return (NULL);
	
	*p = '\0';
	
	return (str);
}

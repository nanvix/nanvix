/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * string/strncpy.c - strncpy() implementation.
 */

#include <sys/types.h>

/*
 * Copies a fixed-size string.
 */
char *strncpy(char *dest, const char *src, size_t n)
{
	char *p1;       /* Indexes dest. */
	const char *p2; /* Indexes src.  */
	
	p1 = dest;
	p2 = src;
	
	/* Copy string. */
	while (n > 0)
	{
		if (*p2 == '\0')
			break;
			
		*p1++ = *p2++;
		n--;
	}
	
	/* Fill with null bytes. */
	while (n > 0)
	{
		 *p1++ = '\0';
		 n--;
	}
	
	return (dest);
}

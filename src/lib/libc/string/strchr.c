/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * strchr.c - strchr() implementation.
 */

#include <stdlib.h>

/*
 * Schans for a character in a string.
 */
char *strchr(const char *str, int c)
{
	/* Scan string. */
	while (*str != '\0')
	{
		/* Found. */
		if (*str == c)
			return ((char*)str);
		
		str++;
	}
	
	return ((c == '\0') ? (char *) str : NULL);
}

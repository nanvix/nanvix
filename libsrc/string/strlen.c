/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stirng/strlen.c - strlen() implementation.
 */

#include <sys/types.h>

/*
 * Returns the length of a string.
 */
size_t strlen(const char * str)
{
	const char *p;
	
	/* Count the number of characters. */
	for (p = str; *p != '\0'; p++)
		/* No operation.*/;
	
	return (p - str);
}


/* Copyright (C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * lib/kstrlen.c - kernel strlen().
 */

#include <nanvix/const.h>

/*
 * Copies a string.
 */
PUBLIC char *kstrcpy(char *dest, const char *src)
{
	char *p;
	
	p = dest;
	
	/* Copy strings. */
	while (*src != '\0')
		*p++ = *src++;
	*p = '\0';
	
	return (dest);
}

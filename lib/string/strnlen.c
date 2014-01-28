/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * string/strnlen.c - strnlen() implementation.
 */

#include <sys/types.h>

/*
 * Gets the length of a fixed-size string.
 */
size_t strnlen(const char *str, size_t maxlen)
{
	const char *p;
	
	/* Count the number of characters. */
	for (p = str; *p != '\0' && maxlen > 0; p++, maxlen--)
		/* No operation.*/;
	
	return (p - str);
}

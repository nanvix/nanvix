/* Copyright (C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * lib/kstrlen.c - kernel strlen().
 */

#include <nanvix/const.h>
#include <sys/types.h>

/*
 * Returns the length of a string.
 */
PUBLIC size_t kstrlen(const char * str)
{
	const char *p = str;
	
	/* Count the number of characters. */
	while (*p++ != '\0') /* No operation.*/;
	
	return ((p - 1) - str);
}

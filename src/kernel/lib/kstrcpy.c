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
    char *d;
    const char *s;
    
    d = dest;
    s = src;
    
    while(*s != '\0')
		*d++ = *s++;
    *d = '\0';
    
    return (dest);
}

/*
 * Copyright (C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * string/memcpy.c - memcpy() implementation.
 */

#include <sys/types.h>

/*
 * Copy bytes in memory.
 */
void *memcpy (void* dest, const void *src, size_t n)
{
    const char* s;
    char *d;
    
    s = src;
    d = dest;
    
    while (n-- > 0)
    	*d++ = *s++;
 
    return (d);
}

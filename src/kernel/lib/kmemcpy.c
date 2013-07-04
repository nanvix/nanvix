/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * kmemcpy.c - Kernel memcpy()
 */

#include <nanvix/const.h>
#include <sys/types.h>

/*
 * Copy bytes in memory.
 */
PUBLIC void* kmemcpy (void* dest, const void *src, size_t n)
{
    const char* s;
    char *d;
    
    s = src;
    d = dest;
    
    while (n-- > 0)
    	*d++ = *s++;
 
    return (d);
}

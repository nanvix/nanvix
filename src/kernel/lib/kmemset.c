/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * kmemset.c - Kernel memset()
 */

#include <nanvix/const.h>
#include <sys/types.h>

/*
 * Sets bytes in memory.
 */
PUBLIC void *kmemset(void *ptr, int c, size_t n)
{
    unsigned char *p;
    
    p = ptr;
    
    /* Set bytes. */
    while (n-- > 0)
		*p++ = (unsigned char) c;

    return (ptr);	
}

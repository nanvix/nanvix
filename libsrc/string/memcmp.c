/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * string/memcmp.c - memcmp() implementation;
 */

#include <stdlib.h>

/*
 * Compares memory areas.
 */
int memcmp(const void *s1,const void *s2, size_t n)
{
	const unsigned char *p1; /* Indexes s1. */
	const unsigned char *p2; /* Indexes s2. */
	
	p1 = s1;
	p2 = s2;
	
	/* Compare characters. */
	while (n-- > 0)
	{
		if (*p1 != *p2)
			return (*p1 - *p2);
	}
	
	return (0);
}

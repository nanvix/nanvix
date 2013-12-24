/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * lib/kstrncpy.c - Kernel strncpy().
 */

#include <nanvix/const.h>
#include <sys/types.h>

/*
 * Copies part of a string.
 */
PUBLIC char *kstrncpy(char *str1, const char *str2, size_t n)
{
	char *p1;       /* Indexes str1. */
	const char *p2; /* Indexes str2. */
	
	p1 = str1;
	p2 = str2;
	
	/* Copy string. */
	while (n > 0)
	{
		if (*p2 == '\0')
			break;
			
		*p1++ = *p2++;
		n--;
	}
	
	/* Fill with null bytes. */
	while (n > 0)
	{
		 *p1++ = '\0';
		 n--;
	}
	
	return (str1);
}

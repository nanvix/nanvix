/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * lib/kstrncmp.c - Kernel strncmp().
 */

#include <nanvix/const.h>
#include <sys/types.h>

/*
 * Compares part of two strings.
 */
PUBLIC int kstrncmp(const char *str1, const char *str2, size_t n)
{
	/* Compare strings. */
	while (n-- > 0)
	{
		/* Strings differ. */
		if (*str1 != *str2)
			break;
			
		/* End of string. */
		if (*str1 == '\0')
			return (0);
		
		str1++;
		str2++;
	}
	
	return (*str1 - *str2);
}

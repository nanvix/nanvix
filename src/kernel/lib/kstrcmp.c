/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * lib/kstrcmp.c - Kernel strcmp().
 */

#include <nanvix/const.h>

/*
 * Compares two strings.
 */
PUBLIC int kstrcmp(const char *str1, const char *str2)
{
	/* Compare strings. */
	while (*str1 == *str2)
	{
		/* End of string. */
		if (*str1 == '\0')
			return (0);
		
		str1++;
		str2++;
	}
	
	return (*str1 - *str2);
}

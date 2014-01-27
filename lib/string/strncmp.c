/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * string/strncmp.c - strncmp() implementation.
 */

#include <sys/types.h>

/*
 * Compares two strings.
 */
int strncmp(const char *str1, const char *str2, size_t n)
{
	/* Compare strings. */
	while (n > 0)
	{
		/* Strings differ. */
		if (*str1 != *str2)
			return ((*(unsigned char *)str1 < *(unsigned char *)str2) ? -1 : 1);
		
		/* End of string. */
		else if (*str1 == '\0')
			return (0);
		
		str1++;
		str2++;
		n--;
	}
	
	return (0);
}

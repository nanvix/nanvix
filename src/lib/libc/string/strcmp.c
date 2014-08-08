/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * string/strcmp.c - strcmp() implementation.
 */

/*
 * Compares two strings.
 */
int strcmp(const char *str1, const char *str2)
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
	
	return ((*(unsigned char *)str1 < *(unsigned char *)str2) ? -1 : 1);
}

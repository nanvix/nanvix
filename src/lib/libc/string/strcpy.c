/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * string/strcpy.c - strcpy() implementation.
 */

/*
 * Copies a string.
 */
char *strcpy(char *dest, const char *src)
{
	char *p;
	
	p = dest;
	
	/* Copy strings. */
	while (*src != '\0')
		*p++ = *src++;
	*p = '\0';
	
	return (dest);
}

/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdlib/getenv.c - getenv() implementation.
 */

#include <unistd.h>
#include <stdlib.h>
#include <string.h>

/*
 * Gets value of an environment variable.
 */
char *getenv(const char *name)
{
	char **p;   /* Working environment variable. */
	int length; /* Environment variable name.    */
	
	length = strlen(name);
	
	/* Search for environment variable. */
	for (p = environ; *p != NULL; p++)
	{
		/* Found. */
		if (!strncmp(name, *p, length))
			return (*p + length + 1);
	}
	
	return (NULL);
}

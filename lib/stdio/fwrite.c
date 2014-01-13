/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/fwrite.c - fwrite() implementation.
 */

#include <sys/types.h>
#include <stdio.h>

/*
 * Writes raw data to a file.
 */
size_t fwrite(const void *ptr, size_t size, size_t nitems, FILE *stream)
{
	size_t i, j;   /* Loop indexes. */
	const char *p; /* Read pointer. */
	
	i = 0;
	p = ptr;
	
	/* Write data. */
	while (i < nitems)
	{
		j = 0;
		
		while (j < size)
		{
			/* Error while writing. */
			if (fputc((int)*p, stream) == EOF)
				return (i);
			
			j++; p++;
		}
		
		i++;
	}
	
	return (i);
}

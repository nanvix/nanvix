/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/fclose.c - fclose() implementation.
 */

#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

/*
 * Closes a file stream.
 */
int fclose(FILE *stream)
{
	fflush(stream);

	/* Failed to close file. */
	if (close(stream->fd) == -1)
		return (EOF);
	
	/* Release file stream. */
	if (stream->flags & _IOMYBUF)
		free(stream->buf);
	stream->flags = 0;
	
	return (0);
}

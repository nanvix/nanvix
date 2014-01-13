/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/ffill.c - ffill() implementation.
 */

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include "stdio.h"

/*
 * Internal ffill().
 */
static int _ffill(FILE *stream)
{
	char *ptr;       /* Read pointer.         */
	const char *end; /* End of data read.     */
	ssize_t count;   /* Number of bytes read. */
	
	/* Nothing to be done. */
	if (stream->nread < 0)
		return (0);
	
	/* End of file reached. */
	if (stream->eof)
		return (EOF);
		
	ptr = &stream->buffer[stream->nread];
	end = &stream->buffer[stream->bufsz];
	
	/* Fill up input buffer. */
	for (ptr = &stream->buffer[stream->nread]; ptr < end; )
	{
		count = read(stream->fd, ptr, end - ptr);
		
		/* End of file reached? */
		if (count == 0)
		{
			stream->eof = 1;
			return (EOF);
		}
			
		/* Read error? */
		if (count < 0)
		{
			stream->error = 1;
			return (EOF);
		}
			
		ptr += count;
		stream->nread += count;
	}
		
	stream->nread = 0;
	
	return (0);
}

/*
 * Fills a file stream.
 */
int ffill(FILE *stream)
{
	FILE *f; /* Working file. */
	int ret; /* Return value. */
	
	/* Flushes all file streams. */
	if (stream == NULL)
	{
		ret = 0;
		
		for (f = &streams[0]; f < &streams[FOPEN_MAX]; f++)
		{
			if ((f->fd < 0) || (ACCMODE(f->omode) == O_WRONLY))
				continue;
			
			ret |= _ffill(f);
		}
		
		return (ret);
	}
	
	/* File is not readable. */
	if (ACCMODE(f->omode) == O_WRONLY)
	{
		errno = EBADF;
		stream->error = 1;
		return (EOF);
	}
	
	return (_ffill(stream));
}

/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/flush.c - fflush() implementation.
 */

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include "stdio.h"

/*
 * Internal fflush().
 */
int _fflush(FILE *stream)
{
	char *ptr;       /* Write pointer.           */
	const char *end; /* End of data.             */
	ssize_t count;   /* Number of bytes written. */
	
	/* Nothing to be done. */
	if (stream->nwritten <= 0)
		return (0);
	
	end = &stream->buffer[stream->nwritten];
	
	/* Write data to underlying file. */
	for (ptr = stream->buffer; ptr < end; )
	{
		count = write(stream->fd, ptr, end - ptr);
			
		/* Write operation has failed? */
		if (count < 0)
		{
			stream->error = 1;
			return (EOF);		
		}
			
		ptr += count;
	}
			
	/* Reset output buffer. */
	stream->nwritten = 0;
	
	return (0);
}

/*
 * Flushes a file stream.
 */
int fflush(FILE *stream)
{
	FILE *f; /* Working file. */
	int ret; /* Return value. */
	
	/* Flushes all file streams. */
	if (stream == NULL)
	{
		ret = 0;
		
		for (f = &streams[0]; f < &streams[FOPEN_MAX]; f++)
		{
			if ((f->fd < 0) || (ACCMODE(f->omode) == O_RDONLY))
				continue;
			
			ret |= _fflush(f);
		}
		
		return (ret);
	}
	
	/* File is not writable. */
	if (ACCMODE(f->omode) == O_RDONLY)
	{
		errno = EBADF;
		stream->error = 1;
		return (EOF);
	}
	
	return (_fflush(stream));
}

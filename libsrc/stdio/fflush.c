/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/fflush.c - fflush() implementation.
 */

#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include "stdio.h"

/*
 * Flushes a stream.
 */
static int do_fflush(FILE *stream)
{
	char *buf; /* Buffer.              */
	ssize_t n; /* Byte count to flush. */
	
	/* Not buffered. */
	if (stream->flags & _IONBF)
		return (0);
	
	/* Not writable. */
	if (!(stream->flags & _IOWRITE))
		return (0);
	
	/* No buffer assigned. */
	if ((buf = stream->buf) == NULL)
		return (0);
	
	/* Nothing to flushed. */
	if ((n = stream->ptr - buf) == 0)
		return (0);
	
	/* Reset buffer. */
	stream->ptr = buf;
	stream->count = (stream->flags & _IOLBF) ? 0 : stream->bufsiz - 1;
	
	/* Flush. */
	if (write(fileno(stream), buf, n) != n)
	{
		stream->flags |= _IOERROR;
		return (EOF);
	}
	
	return (0);
}

/*
 * Flushes a file stream.
 */
int fflush(FILE *stream)
{
	int ret;
	
	/* Flush all streams. */
	if (stream == NULL)
	{
		ret = 0;
		
		for (stream = &streams[0]; stream < &streams[FOPEN_MAX]; stream++)
			ret |= do_fflush(stream);
		
		return (ret);
	}
	
	return (do_fflush(stream));
}

/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/getc.c - getc() implementation.
 */

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include "stdio.h"

/*
 * Reads a character from a file.
 */
int getc(FILE *stream)
{	
	int c;     /* Read character.      */
	int count; /* Characters to write. */
	char *buf; /* Buffer.              */
	
	/* Buffer is not empty. */
	if (--stream->nread >= 0)
		return (*stream->ptr++ & 0xff);
	
	/* File is not readable. */
	if (!(stream->flags & _IOREAD))
		return (EOF);
	
	/* Flush data buffer before reading. */
	if ((stream->flags & _IOWRITING) && !(stream->flags & _IONBF))
	{
		if ((buf = stream->buf) != NULL)
		{
			if ((count = (stream->ptr - buf)) > 0)
			{
				/* Failed. */
				if (write(stream->fd, buf, count) != count)
				{
					stream->flags |= _IOERROR;
					return (EOF);
				}
			}
		}
		
		/* Reset buffer. */
		stream->nwritten = -1;
		stream->ptr = buf;
		stream->flags &= ~_IOWRITING;
		stream->flags |= _IOREADING;
	}
	
again:
	
	/* Not buffered. */
	if (stream->flags & _IONBF)
	{
		/* Reset buffer. */
		stream->flags |= _IOWRITING;
		stream->nread = 0;
		
		/* Setup read parameters. */
		count = 1;
		buf = (char *)&c;
	}
	
	/* Buffered. */
	else
	{
		/* Assign buffer. */
		if ((buf = stream->buf) == NULL)
		{
			/* Failed to allocate buffer. */
			if ((buf = malloc(BUFSIZ)) == NULL)
			{
				stream->flags &= ~(_IOLBF | _IOFBF);
				stream->flags |= _IONBF;
				goto again;
			}
			
			/* Initialize buffer. */
			stream->flags |= _IOMYBUF | _IOREADING;
			stream->buf = buf;
			stream->ptr = buf;
			stream->bufsiz = BUFSIZ;
			stream->nread = 0;
		}
		
		/* Setup read parameters. */
		count = BUFSIZ;
	}
	
	stream->nread = read(stream->fd, buf, count);
	
	/* Reset buffer. */
	stream->ptr = buf;
	
	/* Failed to read. */
	if (--stream->nread < 0)
	{
		stream->flags |= (stream->nread == -1) ? _IOEOF : _IOERROR;
		return (EOF);
	}
	
	return (*stream->ptr++ & 0xff);
}

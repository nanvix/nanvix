/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/putc.c - putc() implementation.
 */

#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include "stdio.h"

/*
 * Writes a character to a file.
 */
int putc(int c, FILE *stream)
{
	int n;     /* Characters actually written. */
	int count; /* Characters to write.         */
	char *buf; /* Buffer.                      */
	
	/* Buffer is not full. */
	if (--stream->nwritten >= 0)
		return (*stream->ptr++ = c);
	
	/* File is not writable. */
	if (!(stream->flags & _IOWRITE))
		return (EOF);
	
	/* Discard data buffer writing. */
	if (stream->flags & _IOREADING)
	{
		/* Synchronize file position. */
		if ((n = stream->nread) > 0)
		{
			/* Failed. */
			if (lseek(stream->fd, -n, SEEK_CUR) < 0)
			{
				stream->flags |= _IOERROR;
				return (EOF);
			}
		}
			
		/* Reset buffer. */
		stream->nread = -1;
		stream->ptr = stream->buf;
		stream->flags &= ~_IOREADING;
		stream->flags |= _IOWRITING;
	}
	
	/* Synchronize file position. */
	if ((stream->flags & _IOAPPEND) && (stream->flags | _IOSYNC))
	{
		/* Failed. */
		if (lseek(stream->fd, 0, SEEK_END) < 0)
		{
			stream->flags |= _IOERROR;
			return (EOF);
		}
	}

again:
	count = n = 0;

	/* Not buffered. */
	if (stream->flags & _IONBF)
	{
		/* Reset buffer. */
		stream->flags |= _IOWRITING;
		stream->nwritten = 0;
		
		n = write(stream->fd, &c, count = 1);
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
			stream->flags |= _IOMYBUF | _IOWRITING;
			stream->buf = buf;
			stream->ptr = buf;
			stream->bufsiz = BUFSIZ;
		}
		
		*stream->ptr++ = c;
		
		/* Line buffered. */
		if (stream->flags & _IOLBF)
		{
			/* Reset buffer. */
			stream->nwritten = 0;
			
			/* Flush buffer. */
			if ((stream->ptr == (buf + stream->bufsiz)) || (c == '\n'))
			{			
				n = write(stream->fd, buf, count = stream->ptr - buf);
				stream->ptr = buf;
			}
		}
		
		/* Fully buffered. */
		else
		{	
			/* Reset buffer. */
			stream->nwritten = stream->bufsiz - 1;
			
			/* Flush buffer. */
			if (stream->ptr == (buf + stream->bufsiz))
			{
				n = write(stream->fd, buf, count = stream->ptr - buf);
				stream->ptr = buf;
			}
		}
	}
		
	/* Failed to write. */
	if (n != count)
	{
		stream->flags |= _IOERROR;
		return (EOF);
	}
	
	return (c);
}

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
	if (--stream->count >= 0)
		return (*stream->ptr++ = c);
	
	/* Now writing. */
	if (stream->flags & _IORW)
	{
		stream->flags &= ~_IOREAD;
		stream->flags |= _IOWRITE;
	}
	
	/* File is not writable. */
	if (!(stream->flags & _IOWRITE))
		return (EOF);

	/* Synchronize file position. */
	if (!(stream->flags & _IOEOF))
	{
		if ((stream->flags & (_IOSYNC | _IOAPPEND)) == (_IOSYNC | _IOAPPEND))
		{
			/* Failed. */
			if (lseek(stream->fd, 0, SEEK_END) < 0)
			{
				stream->flags |= _IOERROR;
				return (EOF);
			}
		}
	}

again:
	count = n = 0;

	/* Not buffered. */
	if (stream->flags & _IONBF)
	{
		/* Reset buffer. */
		stream->count = 0;
		
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
			stream->flags |= _IOMYBUF;
			stream->buf = buf;
			stream->ptr = buf;
			stream->bufsiz = BUFSIZ;
		}
		
		/* Line buffered. */
		if (stream->flags & _IOLBF)
		{
			*stream->ptr++ = c;
		
			/* Reset buffer. */
			stream->count = 0;
			
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
			/* Flush buffer. */
			if (stream->ptr == (buf + stream->bufsiz))
			{
				n = write(stream->fd, buf, count = stream->bufsiz);
				stream->ptr = buf;
			}
			
			*stream->ptr++ = c;
		
			/* Reset buffer. */
			stream->count = stream->bufsiz - 1;
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

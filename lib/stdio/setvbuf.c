/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/setvbuf.c - setvbuf() implementation.
 */

#include <sys/types.h>
#include <errno.h>
#include <stdlib.h>
#include <stdio.h>

/*
 * Assigns a buffer to a stream.
 */
int setvbuf(FILE *stream, char *buf, int type, size_t size)
{
	/* Invalid file stream. */
	if (stream->flags == 0)
		return (errno = EBADF);
	
	/* Buffer already assigned. */
	if (stream->buf != NULL)
		return (errno = EBUSY);
	
	/* Not buffered. */
	if (type == _IONBF)
	{
		stream->flags &= ~(_IOLBF | _IOFBF);
		stream->flags |= _IONBF;
		stream->count = 0;
	}
	
	/* Buffered. */
	else
	{
		/* Invalid buffer size. */
		if (size == 0)
			return (errno = EINVAL);
		
		/* Assign own buffer. */
		if (buf == NULL)
		{
			/* Failed to allocate buffer. */
			if ((buf = malloc(size)) == NULL)
				return (errno);
			
			stream->flags |= _IOMYBUF;
		}
		
		stream->flags &= ~(_IOFBF | _IONBF | _IOFBF);
		stream->buf = buf;
		stream->ptr = buf;
		stream->bufsiz = size;
		
		/* Setup write buffer. */
		if (stream->flags & _IOWRITE)
		{
			if (type == _IOLBF)
			{
				stream->flags |= _IOLBF;
				stream->count = 0;
			}
			
			else
			{
				stream->flags |= _IOFBF;
				stream->count = stream->bufsiz - 1;
			}
		}
		
		/* Setup read buffer. */
		else
		{
			stream->flags |= (type == _IOLBF) ? _IOLBF : _IOFBF;
			stream->count = 0;
		}
	}
	
	return (0);
}

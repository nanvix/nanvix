/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/fputc.c - fputc() implementation.
 */

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include "stdio.h"

/*
 * Writes a character to a file.
 */
int fputc(int ch, FILE *stream)
{
	int flush;
	
	/* File is not writable. */
	if (ACCMODE(stream->omode) == O_RDONLY)
	{
		errno = EBADF;
		stream->error = 1;
		return (EOF);
	}
	
	/* Initialize buffer. */
	if (stream->buffer == NULL)
	{
		stream->buffer = malloc(BUFSIZ);
		
		/* Failed to allocate stream buffer. */
		if (stream->buffer == NULL)
			return (EOF);
		
		stream->bufsz = BUFSIZ;
		stream->nread = -1;
		stream->nwritten = 0;
		stream->buf_mode = _IOFBF;
		stream->usr_buf = 0;
	}
	
	/* Invalidate read buffer. */
	if (stream->nread >= 0)
	{
		stream->ptr -= stream->nread;
		if (stream->nread > 0)
			lseek(stream->fd, stream->ptr, SEEK_SET);
		stream->nwritten = 0;
		stream->nread = -1;
	}
	
	/* Flush buffer before writing to it.  */
	else if (WBUF_FULL(stream))
	{
		/* Error while flushing? */
		if (fflush(stream) == EOF)
			return (EOF);
	}
	
	/* Append? */
	if (stream->omode & O_APPEND)
	{
		stream->eof = 0;
		stream->ptr = stream->size;
	}
	
	/* Write. */
	stream->buffer[stream->nwritten++] = (char) ch;
	stream->ptr++;
	
	/* Update file size. */
	if (stream->ptr > stream->size)
		stream->size = stream->ptr;
	
	/* Flush buffer? */
	flush = 0;
	switch (stream->buf_mode)
	{
		/* Fall through. */
		case _IONBF: 
		case _IOLBF:
			if (ch != '\n')
				break;
		case _IOFBF:
			if  (!WBUF_FULL(stream))
				break;
		flush = 1;
	}
	
	/* Flush buffer. */
	if ((flush) && (fflush(stream) == EOF))
		return (EOF);
		
	return (ch);
}

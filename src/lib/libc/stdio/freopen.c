/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/freopen.c - freopen() implementation.
 */

#include <sys/stat.h>
#include <errno.h>
#include <fcntl.h>
#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

#include "stdio.h"

/*
 * Reopens a file stream.
 */
FILE *freopen(const char *filename, const char *mode, FILE *stream)
{	
	int fd;    /* File descriptor.  */
	int flags; /* Stream flags.     */
	int oflags;/* Flags for open(). */
	
	/* File permissions. */
	#define MAY_READ (S_IRUSR | S_IRGRP | S_IROTH)
	#define MAY_WRITE (S_IWUSR | S_IWGRP | S_IWOTH)
	
	fclose(stream);
	
	/* Bad opening mode. */
	if ((flags = _sflags(mode, &oflags)) == 0)
		return (NULL);
	
	/* Failed to open file. */
	if ((fd = open(filename, oflags, MAY_READ | MAY_WRITE)) == -1)
		return (NULL);
	
	stream->fd = fd;
	stream->flags = flags;
	stream->buf = NULL;
	stream->count = 0;
	
	return (stream);	
}

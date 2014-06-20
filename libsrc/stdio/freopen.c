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

/*
 * Reopens a file stream.
 */
FILE *freopen(const char *filename, const char *mode, FILE *stream)
{	
	int rw;    /* Read/Write?       */
	int fd;    /* File descriptor.  */
	int flags; /* Stream flags.     */
	int oflag; /* Flags for open(). */
	
	/* File permissions. */
	#define MAY_READ (S_IRUSR | S_IRGRP | S_IROTH)
	#define MAY_WRITE (S_IWUSR | S_IWGRP | S_IWOTH)
	
	fclose(stream);
	
	rw = (mode[1] == '+');
	flags = _IOFBF;
	
	/* Get open mode. */
	switch (*mode)
	{
		/* Opend file for reading. */
		case 'r':
			oflag = (rw) ? O_RDWR : O_RDONLY;
			break;
		
		/* Open file for writing. */
		case 'w':
			oflag = O_CREAT | O_TRUNC | (rw) ? O_RDWR : O_WRONLY;
			break;
			
		/* Open file for appending. */
		case 'a':
			flags |= _IOAPPEND | _IOSYNC;
			oflag = O_CREAT | (rw) ? O_RDWR : O_WRONLY;
			break;
			
		default:
			errno = EINVAL;
			return (NULL);
	}
	
	/* Failed to open file. */
	if ((fd = open(filename, oflag, MAY_READ | MAY_WRITE)) == -1)
		return (NULL);
	
	stream->fd = fd;
	stream->flags = (rw) ? _IORW : (*mode == 'r') ? _IOREAD : _IOWRITE | flags;
	stream->buf = NULL;
	stream->count = 0;
	
	return (stream);	
}

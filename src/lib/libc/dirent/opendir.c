/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * dirent/opendir.c - opendir() implementation.
 */

#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdlib.h>
#include "dirent.h"

/*
 * Opens a directory stream.
 */
DIR *opendir(const char *dirname)
{
	int fd;    /* Underlying file descriptor. */
	DIR *dirp; /* Working directory stream.   */
	
	/* 
	 * Find an empty slot in the
	 * directory streams table. 
	 */
	for (dirp = &dirs[0]; dirp < &dirs[OPEN_MAX]; dirp++)
	{
		/* Found. */
		if ((!dirp->flags & _DIR_VALID))
			goto found;
	}
	
	errno = EMFILE;
	return (NULL);

found:
	
	/* Open directory. */
	if ((fd = open(dirname, O_RDONLY)) < 0)
		return (NULL);
	
	dirp->fd = fd;
	dirp->flags = _DIR_VALID;
	dirp->count = 0;
	dirp->ptr = NULL;
	dirp->buf = NULL;
	
	return (dirp);
}

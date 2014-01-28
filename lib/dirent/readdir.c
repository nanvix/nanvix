/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * dirent/readdir.c - readdir() implementation.
 */

#include <nanvix/fs.h>
#include <dirent.h>
#include <stdlib.h>
#include <unistd.h>

/*
 * Reads a directory.
 */
struct dirent *readdir(DIR *dirp)
{
	struct dirent *dp;  /* Working directory entry. */
	struct dirent *buf; /* Buffer.                  */

again:

	/* Get next non-empty directory entry. */
	while (--dirp->count >= 0)
	{
		dp = dirp->ptr++;
		
		/* Found. */
		if (dp->d_ino != INODE_NULL)
			return (dp);
	}
	
	/* Allocate buffer. */
	if ((buf = dirp->buf) == NULL)
	{
		buf = malloc(_DIR_BUFSIZ);
	
		/* Failed to allocate buffer. */
		if (buf == NULL)
			return (NULL);
	}
	
	dirp->count = read(dirp->fd, buf, _DIR_BUFSIZ)/_SIZEOF_DIRENT;
	
	/* Reset buffer. */
	dirp->ptr = buf;
	dirp->buf = buf;
	
	/*
	 * Buffer is full again, so go back and try
	 * to get a non-empty directory entry.
	  */
	if (dirp->count > 0)
		goto again;	
	
	return (NULL);
}

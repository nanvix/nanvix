/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * dirent/readdir.c - readdir() implementation.
 */

#include <nanvix/fs.h>
#include <dirent.h>
#include <stdlib.h>
#include <unistd.h>
#include <stdio.h>

/*
 * Reads a directory.
 */
struct dirent *readdir(DIR *dirp)
{
	struct dirent *buf; /* Buffer.         */
	struct dirent *dp;  /* Working dirent. */
	
	/* End of directory. */
	if (dirp->flags & _DIR_EOD)
		return (NULL);
	
	do
	{
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
		
		/* Error while reading. */
		if (dirp->count <= 0)
		{
			/* End of directory. */
			if (dirp->count == 0)
				dirp->flags |= _DIR_EOD;
			
			return (NULL);
		}
	} while (1);

	/* Never gets here. */
	return (NULL);
}

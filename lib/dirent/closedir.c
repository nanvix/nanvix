/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * dirent/closedir.c - closedir() implementation.
 */

#include <dirent.h>
#include <stdlib.h>
#include <unistd.h>

/*
 * Closes a directory stream.
 */
int closedir(DIR *dirp)
{
	/* Close underlying directory. */
	if (close(dirp->fd) < 0)
		return (-1);
	
	/* Invalidate directory stream. */
	if (dirp->buf != NULL)
		free(dirp->buf);
	dirp->flags &= ~_DIR_VALID;
	
	return (0);
}

/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * dirent/rewinddir.c - rewinddir() implementation.
 */

#include <dirent.h>

/*
 * Rewinds a directory stream.
 */
void rewinddir(DIR *dirp)
{
	/* Invalidate buffer. */
	dirp->count = 0;
	dirp->buf = dirp->ptr;
	dirp->flags &= ~_DIR_EOD;
}


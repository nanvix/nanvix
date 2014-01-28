/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * dirent/dirent.c - Private dirent library variables and functions.
 */

#include <dirent.h>
#include <limits.h>

/* Directory stream table. */
DIR dirs[OPEN_MAX];

/*
 * Dirent library house keeping.
 */
void dirent_cleanup(void)
{
	DIR *dirp;
	
	/* Closes all directory streams. */
	for (dirp = &dirs[0]; dirp < &dirs[OPEN_MAX]; dirp++)
	{
		/* Found. */
		if (dirp->flags & _DIR_VALID)
			closedir(dirp);
	}
}

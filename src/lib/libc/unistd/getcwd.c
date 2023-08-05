/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * This file is part of Nanvix.
 *
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <dirent.h>
#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>

/*
 * Looks up for a specific directory entry.
 */
static struct dirent *getdirent(DIR *dirp, ino_t ino)
{
	struct dirent *dp;

	/*
	 * No need to rewind. We assume we are
	 * at the begin of the directory.
	 */

	do
	{
		/* check this directory entry. */
		if ((dp = readdir(dirp)) != NULL)
		{
			/* Not found. */
			if (dp->d_ino != ino)
				continue;

			return (dp);
		}
	} while (dp != NULL);

	return (NULL);
}

/*
 * Prepends a directory name to a path.
 */
static int prepend(char *pathname, const char *dirname, size_t n)
{
	char *p;        /* Write pointer.         */
	size_t pathlen; /* Pathname length.       */
	size_t dirlen;  /* Directory name length. */

	pathlen = strlen(pathname);
	dirlen = strlen(dirname);
	if (dirlen > NAME_MAX)
		dirlen = NAME_MAX;

	/* Out of space. */
	if (pathlen + dirlen + 2 > n)
		return (errno = ERANGE);

	/* Push pathname. */
	for (p = pathname + pathlen;  p >= pathname; p--)
		*(p + dirlen + 1) = *p;

	memcpy(pathname, "/", 1);
	memcpy(pathname + 1, dirname, dirlen);

	return (0);
}

/*
 * Gets the pathname of the current working directory.
 */
char *getcwd(char *buf, size_t size)
{
	ino_t cino;            /* Current dir inode number.  */
	dev_t cdev;            /* Current dir inode pointer. */
	ino_t rino;            /* Root dir inode number.     */
	dev_t rdev;            /* Root dir device number.    */
	DIR *dirp;             /* Working directory.         */
	struct stat st;        /* Working directory stat.    */
	struct dirent *dp;     /* Working directory entry.   */
	char curdir[PATH_MAX]; /* Current directory.         */

	/* Invalid size. */
	if (size == 0)
	{
		errno = EINVAL;
		return (NULL);
	}

	buf[0] = '\0';
	strcpy(curdir, ".");

	/*
	 * Get root inode and device numbers
	 * so we know where we should stop.
	 */
	if (stat("/", &st) < 0)
		goto error0;

	rino = st.st_ino;
	rdev = st.st_dev;

	/* Get current working directory pathname. */
	while (1)
	{
		/*
		 * Remember current directory inode
		 * and device number to get it name later.
		 */
		if (stat(curdir, &st) < 0)
			goto error0;

		cino = st.st_ino;
		cdev = st.st_dev;

		/* Done. */
		if ((cino == rino) && (cdev == rdev))
			break;

		/* Open upper directory. */
		strcat(curdir, "/..");
		if ((dirp = opendir(curdir)) == NULL)
			goto error0;

		/* Get lower directory name. */
		dp = getdirent(dirp, cino);
		if (prepend(buf, dp->d_name, size))
			goto error1;

		closedir(dirp);
	}

	/*
	 * Special case when current working directory
	 * is the root directory.
	 */
	if (buf[0] == '\0')
	{
		/* Start from root*/
		if (prepend(buf, "", size))
			goto error0;
	}

	return (buf);

error1:
	closedir(dirp);
error0:
	return (NULL);
}

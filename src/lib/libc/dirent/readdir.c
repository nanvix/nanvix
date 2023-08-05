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

		dirp->count = read(dirp->fd, buf, _DIR_BUFSIZ)/sizeof(struct dirent);

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

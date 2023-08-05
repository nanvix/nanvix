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

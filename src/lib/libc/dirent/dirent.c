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

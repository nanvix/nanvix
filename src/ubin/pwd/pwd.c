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

#include <errno.h>
#include <limits.h>
#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

/* Software versioning. */
#define VERSION_MAJOR 1 /* Major version. */
#define VERSION_MINOR 0 /* Minor version. */

/*
 * Prints the name of the working directory
 */
int main(int argc, char *const argv[])
{
	char pathname[PATH_MAX];

	((void)argc);
	((void)argv);

	/*
	 * Get current working directory name. getcwd() will
	 * do all the dirty job. It will contruct the current
	 * working directory name from bottom to up, that is,
	 * towards the root directory.
	 */
	if (getcwd(pathname, PATH_MAX) == NULL)
	{
		fprintf(stderr, "pwd: cannot getcwd\n");
		return (EXIT_FAILURE);
	}

	printf("%s\n", pathname);

	return (EXIT_SUCCESS);
}

/*
 * Copyright(C) 2011-2017 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2017-2017 Romane Gallier <romanegallier@gmail.com>
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
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>


/* Software versioning. */
#define VERSION_MAJOR 1 /* Major version. */
#define VERSION_MINOR 0 /* Minor version. */


static char *device = NULL; 
static char *destination_dir= NULL;

/*
 * Lists contents of a directory
 */
int main(int argc, char *const argv[])
{
	/* Missing arguments. */
	if (argc < 2)
	{
		printf ("Missing arguments\n");
		return (EXIT_FAILURE);
	}

	/* Retrieve parameters. */
	device = argv[1];
	destination_dir = argv[2];
	
	/* Mount file system */
	if (mount(device, destination_dir))
		printf("Failed to mount\n");
	
	return (EXIT_SUCCESS);
}


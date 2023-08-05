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

#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <unistd.h>

#include "minix.h"
#include "util.h"

/**
 * @brief Prints program usage and exits.
 */
static void usage(void)
{
	printf("usage: mknod.minix ");
	printf("<input file> <file name> <mode> <type> <minor> <major> <uid> <gid>\n");
	exit(EXIT_SUCCESS);
}

/**
 * @brief Returns device number.
 *
 * @param type  Device type.
 * @param major Major number.
 * @param minor Minor number.
 *
 * @returns The device number.
 */
static inline uint16_t devnum(char type, unsigned major, unsigned minor)
{
	return (((major & 0xf) << 8)|((minor & 0xf) << 4) | (type == 'c' ? 0 : 1));
}

/**
 * @brief Creates a special file in a Minix file system.
 */
int main(int argc, char **argv)
{
	const char *pathname;              /* Directory where create file.     */
	uint16_t num;                      /* Working inode number.            */
	struct d_inode *dip;               /* Working inode.                   */
	char filename[MINIX_NAME_MAX + 1]; /* Working file name.               */
	unsigned mode;                     /* Access mode of the special file. */
	unsigned major, minor;             /* Major and minor numbers.         */
	char type;                         /* Character type.                  */

	/* Wrong usage. */
	if (argc != 9)
		usage();

	pathname = argv[2];
	sscanf(argv[3], "%o", &mode);
	sscanf(argv[4], "%c", &type);
	sscanf(argv[5], "%u", &minor);
	sscanf(argv[6], "%u", &major);

	minix_mount(argv[1]);
	num = minix_inode_dname(pathname, filename);
	dip = minix_inode_read(num);
	minix_mknod(dip, filename, mode, devnum(type, major, minor), atoi(argv[7]), atoi(argv[8]));
	minix_inode_write(num, dip);
	minix_umount();

	return (EXIT_SUCCESS);
}

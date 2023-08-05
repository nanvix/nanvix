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
#include <unistd.h>

#include "minix.h"
#include "util.h"

/**
 * @brief Prints program usage and exits.
 *
 * @details Prints program usage and exits.
 */
static void usage(void)
{
	printf("usage: mkdir.minix <input file> <directory> <uid> <gid>\n");
	exit(EXIT_SUCCESS);
}

/**
 * @brief Creates a directory in a Minix file system.
 */
int main(int argc, char **argv)
{
	const char *dirname;               /* Directory that shall be created. */
	uint16_t num1, num2;               /* Working inode numbers.           */
	struct d_inode *ip;                /* Working inode.                   */
	char filename[MINIX_NAME_MAX + 1]; /* Working file name.               */

	/* Wrong usage. */
	if (argc != 5)
		usage();

	minix_mount(argv[1]);

	dirname = argv[2];

	/* Traverse file system tree. */
	ip = minix_inode_read(num1 = INODE_ROOT);
	do
	{
		dirname = break_path(dirname, filename);
		num2 = dir_search(ip, filename);

		/* Create directory. */
		if (num2 == INODE_NULL)
			num2 = minix_mkdir(ip, num1, filename, atoi(argv[3]), atoi(argv[4]));

		minix_inode_write(num1, ip);
		ip = minix_inode_read(num1 = num2);
	} while (*dirname != '\0');

	minix_inode_write(num1, ip);

	minix_umount();

	return (EXIT_SUCCESS);
}

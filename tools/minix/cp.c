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

#include <sys/stat.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <unistd.h>

#include "minix.h"
#include "util.h"

/**
 * @brief Safe stat().
 */
void sstat(const char *pathname, struct stat *buf)
{
	int ret;

	ret = stat(pathname, buf);
	if (ret != 0)
		error("cannot stat()");
}

/**
 * @brief Prints program usage and exits.
 */
static void usage(void)
{
	printf("usage: cp.minix <input file> <source file> <dest file> <uid> <gid> \n");
	exit(EXIT_SUCCESS);
}

/**
 * @brief Copies a file to a Minix file system.
 */
int main(int argc, char **argv)
{
	int fd;                 /* File ID of source file.       */
	struct stat stbuf;      /* Buffer for stat().            */
	char *buf;              /* Buffer used for copying.      */
	const char *src, *dest; /* Source and destination files. */
	uint16_t num;           /* Working inode number.         */

	/* Wrong usage. */
	if (argc != 6)
		usage();

	src = argv[2];
	dest = argv[3];

	sstat(src, &stbuf);

	buf = smalloc(stbuf.st_size);
	minix_mount(argv[1]);

	/* Copy file. */
	fd = sopen(src, O_RDONLY);
	sread(fd, buf, stbuf.st_size);
	sclose(fd);
	num = minix_create(dest, stbuf.st_mode, atoi(argv[4]), atoi(argv[5]));
	minix_write(num, buf, stbuf.st_size);

	minix_umount();
	free(buf);

	return (EXIT_SUCCESS);
}

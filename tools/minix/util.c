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

#include <sys/types.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

#include "minix.h"

/**
 * @brief Prints an error message and exits.
 *
 * @param msg Error message to be printed.
 */
void error(const char *msg)
{
	fprintf(stderr, "error: %s\n", msg);
	exit(EXIT_FAILURE);
}

/**
 * @brief Safe open().
 */
int sopen(const char *pathname, int flags)
{
	int fd;

	fd = open(pathname, flags);
	if (fd == -1)
	{
		fprintf(stderr, "cannot open()\n");
		exit(EXIT_FAILURE);
	}

	return (fd);
}

/**
 * @brief Safe close().
 */
void sclose(int fd)
{
	int ret;

	ret = close(fd);
	if (ret == -1)
	{
		fprintf(stderr, "cannot close()\n");
		exit(EXIT_FAILURE);
	}
}

/**
 * @brief Safe lseek().
 */
void slseek(int fd, off_t offset, int whence)
{
	off_t ret;

	ret = lseek(fd, offset, whence);
	if (ret == (-1))
	{
		fprintf(stderr, "cannot lseek()\n");
		exit(EXIT_FAILURE);
	}
}

/**
 * @brief Safe read().
 */
void sread(int fd, void *buf, size_t count)
{
	size_t ret;

	ret = read(fd, buf, count);
	if (ret != count)
	{
		fprintf(stderr, "cannot read()\n");
		exit(EXIT_FAILURE);
	}
}

/**
 * @brief Safe write().
 */
void swrite(int fd, const void *buf, size_t count)
{
	size_t ret;

	ret = write(fd, buf, count);
	if (ret != count)
	{
		fprintf(stderr, "cannot write()\n");
		exit(EXIT_FAILURE);
	}
}

/**
 * @brief Safe malloc();
 */
void *smalloc(size_t n)
{
	void *p;

	p = malloc(n);
	if (p == NULL)
	{
		fprintf(stderr, "cannot malloc()\n");
		exit(EXIT_FAILURE);
	}

	return (p);
}

/**
 * @brief Safe calloc();
 */
void *scalloc(size_t nmemb, size_t n)
{
	void *p;

	p = calloc(nmemb, n);
	if (p == NULL)
	{
		fprintf(stderr, "cannot calloc()\n");
		exit(EXIT_FAILURE);
	}

	return (p);
}

/**
 * @brief Breaks a path.
 *
 * @param pathname Path that shall be broken.
 * @param filename Array where the first path-part should be save.
 *
 * @returns A pointer to the second path-part is returned, so a new call to this
 *          function can be made to parse the remainder of the path.
 */
const char *break_path(const char *pathname, char *filename)
{
	char *p2;       /* Write pointer. */
	const char *p1; /* Read pointer.  */

	p1 = pathname;
	p2 = filename;

	/* Skip those. */
	while (*p1 == '/')
		p1++;

	/* Get file name. */
	while ((*p1 != '\0') && (*p1 != '/'))
	{
		if ((p2 - filename) > MINIX_NAME_MAX)
			error("file name too long");
		*p2++ = *p1++;
	}

	*p2 = '\0';

	return (p1);
}

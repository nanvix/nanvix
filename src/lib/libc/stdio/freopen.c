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
#include <errno.h>
#include <fcntl.h>
#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

#include "stdio.h"

/*
 * Reopens a file stream.
 */
FILE *freopen(const char *filename, const char *mode, FILE *stream)
{
	int fd;    /* File descriptor.  */
	int flags; /* Stream flags.     */
	int oflags;/* Flags for open(). */

	/* File permissions. */
	#define MAY_READ (S_IRUSR | S_IRGRP | S_IROTH)
	#define MAY_WRITE (S_IWUSR | S_IWGRP | S_IWOTH)

	fclose(stream);

	/* Bad opening mode. */
	if ((flags = _sflags(mode, &oflags)) == 0)
		return (NULL);

	/* Failed to open file. */
	if ((fd = open(filename, oflags, MAY_READ | MAY_WRITE)) == -1)
		return (NULL);

	stream->fd = fd;
	stream->flags = flags;
	stream->buf = NULL;
	stream->count = 0;

	return (stream);
}

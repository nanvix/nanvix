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

#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include "stdio.h"

/*
 * Flushes a stream.
 */
static int do_fflush(FILE *stream)
{
	char *buf; /* Buffer.              */
	ssize_t n; /* Byte count to flush. */

	/* Not buffered. */
	if (stream->flags & _IONBF)
		return (0);

	/* Not writable. */
	if (!(stream->flags & _IOWRITE))
		return (0);

	/* No buffer assigned. */
	if ((buf = stream->buf) == NULL)
		return (0);

	/* Nothing to flushed. */
	if ((n = stream->ptr - buf) == 0)
		return (0);

	/* Reset buffer. */
	stream->ptr = buf;
	stream->count = (stream->flags & _IOLBF) ? 0 : stream->bufsiz - 1;

	/* Flush. */
	if (write(fileno(stream), buf, n) != n)
	{
		stream->flags |= _IOERROR;
		return (EOF);
	}

	return (0);
}

/*
 * Flushes a file stream.
 */
int fflush(FILE *stream)
{
	int ret;

	/* Flush all streams. */
	if (stream == NULL)
	{
		ret = 0;

		for (stream = &streams[0]; stream < &streams[FOPEN_MAX]; stream++)
			ret |= do_fflush(stream);

		return (ret);
	}

	return (do_fflush(stream));
}

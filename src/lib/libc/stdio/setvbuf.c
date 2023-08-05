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
#include <errno.h>
#include <stdlib.h>
#include <stdio.h>

/*
 * Assigns a buffer to a stream.
 */
int setvbuf(FILE *stream, char *buf, int type, size_t size)
{
	/* Invalid file stream. */
	if (stream->flags == 0)
		return (errno = EBADF);

	/* Buffer already assigned. */
	if (stream->buf != NULL)
		return (errno = EBUSY);

	/* Not buffered. */
	if (type == _IONBF)
	{
		stream->flags &= ~(_IOLBF | _IOFBF);
		stream->flags |= _IONBF;
		stream->count = 0;
	}

	/* Buffered. */
	else
	{
		/* Invalid buffer size. */
		if (size == 0)
			return (errno = EINVAL);

		/* Assign own buffer. */
		if (buf == NULL)
		{
			/* Failed to allocate buffer. */
			if ((buf = malloc(size)) == NULL)
				return (errno);

			stream->flags |= _IOMYBUF;
		}

		stream->flags &= ~(_IOFBF | _IONBF | _IOFBF);
		stream->flags |= (type == _IOLBF) ? _IOLBF : _IOFBF;
		stream->buf = buf;
		stream->ptr = buf;
		stream->bufsiz = size;
		stream->count = 0;
	}

	return (0);
}

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

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include "stdio.h"

/*
 * Reads a character from a file.
 */
int getc(FILE *stream)
{
	int c;     /* Read character.      */
	int count; /* Characters to write. */
	char *buf; /* Buffer.              */

	/* Buffer is not empty. */
	if (--stream->count >= 0)
		return (*stream->ptr++ & 0xff);

	/* Now reading. */
	if (stream->flags & _IORW)
	{
		stream->flags &= ~_IOWRITE;
		stream->flags |= _IOREAD;
	}

	/* File is not readable. */
	if (!(stream->flags & _IOREAD))
		return (EOF);

	/* End of file reached. */
	if (stream->flags & _IOEOF)
		return (EOF);

again:

	/* Not buffered. */
	if (stream->flags & _IONBF)
	{
		/* Reset buffer. */
		stream->count = 0;

		/* Setup read parameters. */
		count = 1;
		buf = (char *)&c;
	}

	/* Buffered. */
	else
	{
		/* Assign buffer. */
		if ((buf = stream->buf) == NULL)
		{
			/* Failed to allocate buffer. */
			if ((buf = malloc(BUFSIZ)) == NULL)
			{
				stream->flags &= ~(_IOLBF | _IOFBF);
				stream->flags |= _IONBF;
				goto again;
			}

			/* Initialize buffer. */
			stream->flags |= _IOMYBUF;
			stream->buf = buf;
			stream->ptr = buf;
			stream->bufsiz = BUFSIZ;
			stream->count = 0;
		}

		/* Setup read parameters. */
		count = BUFSIZ;
	}

	stream->count = read(fileno(stream), buf, count);

	/* Reset buffer. */
	stream->ptr = buf;

	/* Failed to read. */
	if (--stream->count < 0)
	{
		stream->flags |= (stream->count == -1) ? _IOEOF : _IOERROR;
		return (EOF);
	}

	return (*stream->ptr++ & 0xff);
}

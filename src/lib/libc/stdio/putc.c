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
 * Writes a character to a file.
 */
int putc(int c, FILE *stream)
{
	int n;     /* Characters actually written. */
	int count; /* Characters to write.         */
	char *buf; /* Buffer.                      */

	/* Buffer is not full. */
	if (--stream->count >= 0)
		return (*stream->ptr++ = c);

	/* Now writing. */
	if (stream->flags & _IORW)
	{
		stream->flags &= ~_IOREAD;
		stream->flags |= _IOWRITE;
	}

	/* File is not writable. */
	if (!(stream->flags & _IOWRITE))
		return (EOF);

	/* Synchronize file position. */
	if (!(stream->flags & _IOEOF))
	{
		if ((stream->flags & (_IOSYNC | _IOAPPEND)) == (_IOSYNC | _IOAPPEND))
		{
			/* Failed. */
			if (lseek(fileno(stream), 0, SEEK_END) < 0)
			{
				stream->flags |= _IOERROR;
				return (EOF);
			}
		}
	}

again:
	count = n = 0;

	/* Not buffered. */
	if (stream->flags & _IONBF)
	{
		/* Reset buffer. */
		stream->count = 0;

		n = write(fileno(stream), &c, count = 1);
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
		}

		/* Line buffered. */
		if (stream->flags & _IOLBF)
		{
			*stream->ptr++ = c;

			/* Reset buffer. */
			stream->count = 0;

			/* Flush buffer. */
			if ((stream->ptr == (buf + stream->bufsiz)) || (c == '\n'))
			{
				n = write(fileno(stream), buf, count = stream->ptr - buf);
				stream->ptr = buf;
			}
		}

		/* Fully buffered. */
		else
		{
			/* Flush buffer. */
			if (stream->ptr == (buf + stream->bufsiz))
			{
				n = write(fileno(stream), buf, count = stream->bufsiz);
				stream->ptr = buf;
			}

			*stream->ptr++ = c;

			/* Reset buffer. */
			stream->count = stream->bufsiz - 1;
		}
	}

	/* Failed to write. */
	if (n != count)
	{
		stream->flags |= _IOERROR;
		return (EOF);
	}

	return (c);
}

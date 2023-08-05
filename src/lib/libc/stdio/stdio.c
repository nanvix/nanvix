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

/* File streams table. */
FILE streams[FOPEN_MAX] = {
	{ 0, _IOREAD  | _IOLBF, NULL, NULL, 0, 0 },
	{ 1, _IOWRITE | _IOLBF, NULL, NULL, 0, 0 },
	{ 2, _IOWRITE | _IONBF, NULL, NULL, 0, 0 },
};

/* Standard file streams. */
FILE *stdin = &streams[0];  /* Standard input.  */
FILE *stdout = &streams[1]; /* Standard output. */
FILE *stderr = &streams[2]; /* Standard error.  */

/*
 * Stdio library house keeping.
 */
void stdio_cleanup(void)
{
	FILE *stream;

	/* Close all streams. */
	for (stream = &streams[0]; stream < &streams[FOPEN_MAX]; stream++)
	{
		/* Valid stream. */
		if (stream->flags & (_IORW | _IOREAD | _IOWRITE))
			fclose(stream);
	}
}

/**
 * @brief Finds a file stream that is not in use.
 *
 * @returns A file stream that is not in use. If no such stream is found, NULL
 *          is returned otherwise.
 */
FILE *_getstream(void)
{
	for (FILE *stream = &streams[0]; stream < &streams[FOPEN_MAX]; stream++)
	{
		/* Valid stream. */
		if (!(stream->flags & (_IORW | _IOREAD | _IOWRITE)))
			return (stream);
	}

	return (NULL);
}


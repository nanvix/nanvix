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
#include <errno.h>

/**
 * @brief Returns the (stdio) flags for a given mode.
 *
 * @param mode   Opening mode.
 * @param oflags Opening flags.
 *
 * @returns Flags for an opening mode.
 */
int _sflags(const char *mode, int *oflags)
{
	int flags;

	/* Get open mode. */
	switch (*mode++)
	{
		/* Opend file for reading. */
		case 'r':
			flags = _IOREAD;
			*oflags = O_RDONLY;
			break;

		/* Open file for writing. */
		case 'w':
			flags = _IOWRITE;
			*oflags = O_CREAT | O_TRUNC | O_WRONLY;
			break;

		/* Open file for appending. */
		case 'a':
			flags = _IOWRITE | _IOAPPEND | _IOSYNC;
			*oflags = O_CREAT | O_WRONLY;
			break;

		/* Illegal mode. */
		default:
			errno = EINVAL;
			return (0);
	}

	/* Read/write. */
	if ((*mode == '+') || ((*mode == 'b') && mode[1] == '+'))
	{
		flags = (flags & ~(_IOREAD |_IOWRITE)) | _IORW;
		*oflags = O_RDWR;
	}

	return (flags | _IOFBF);
}

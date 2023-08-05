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

#include <nanvix/syscall.h>
#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>

/*
 * Opens a file.
 */
int open(const char *path, int oflag, ...)
{
	int ret;     /* Return value.      */
	mode_t mode; /* Creation mode.     */
	va_list arg; /* Variable argument. */

	mode = 0;

	if (oflag & O_CREAT)
	{
		va_start(arg, oflag);
		mode = va_arg(arg, mode_t)
		va_end(arg);
	}

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_open),
		  "b" (path),
		  "c" (oflag),
		  "d" (mode)
	);

	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}

	return (ret);
}

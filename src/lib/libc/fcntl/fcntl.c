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
 * Manipulates file descriptor.
 */
int fcntl(int fd, int cmd, ...)
{
	int ret;      /* Return value.      */
	int arg;      /* Creation mode.     */
	va_list varg; /* Variable argument. */

	arg = 0;

	if (cmd == F_DUPFD)
	{
		va_start(varg, arg);
		arg = va_arg(varg, int)
		va_end(varg);
	}

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_fcntl),
		  "b" (fd),
		  "c" (cmd),
		  "d" (arg)
	);

	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}

	return (ret);
}

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
#include <stdarg.h>
#include <stropts.h>
#include <unistd.h>

/*
 * Performs control operations on a device.
 */
int ioctl(int fd, int cmd, ...)
{
	int ret;       /* Return value.      */
	unsigned carg; /* Command argument.  */
	va_list arg;   /* Variable argument. */

	carg = 0;
	if (IOCTL_MAJOR(cmd) & 1)
	{
		va_start(arg, cmd);
		carg = va_arg(arg, unsigned)
		va_end(arg);
	}

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_ioctl),
		  "b" (fd),
		  "c" (cmd),
		  "d" (carg)
	);

	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}

	return (ret);
}

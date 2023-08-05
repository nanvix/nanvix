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
#include <sys/types.h>
#include <errno.h>

/*
 * Moves the read/write file offset.
 */
off_t lseek(int fd, off_t offset, int whence)
{
	__asm__ volatile (
		"int $0x80"
		: "=a" (offset)
		: "0" (NR_lseek),
		  "b" (fd),
		  "c" (offset),
		  "d" (whence)
	);

	/* Error. */
	if (offset < 0)
	{
		errno = -offset;
		return (-1);
	}

	return (offset);
}


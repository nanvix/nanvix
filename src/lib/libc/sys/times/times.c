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
#include <sys/times.h>
#include <sys/types.h>
#include <errno.h>

/**
 * @brief Gets process and waited-for child process times.
 *
 * @param buffer Timing accounting information.
 *
 * @returns Upon successful completion, the elapsed real time, in clock ticks,
 *          since system start-up time is returned. Upon failure, -1 is
 *          returned and errno set to indicate the error.
 */
clock_t times(struct tms *buffer)
{
	clock_t elapsed;

	__asm__ volatile (
		"int $0x80"
		: "=a" (elapsed)
		: "0" (NR_times),
		  "b" (buffer)
	);

	/* Error. */
	if (elapsed < 0)
	{
		errno = -elapsed;
		return (-1);
	}

	return (elapsed);
}

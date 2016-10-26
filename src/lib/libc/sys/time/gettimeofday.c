/*
 * Copyright(C) 2016-2016 Davidson Francis <davidsondfgl@gmail.com>
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
#include <sys/time.h>
#include <errno.h>
#include <reent.h>

/*
 * @brief Get the time expressed as seconds and microseconds since
 * the Epoch.
 *
 * @param tp Timeval structure.
 * @param tzp Timezone structure, should be NULL.
 *
 * @param 0 on success or -1 otherwise.
 */
int gettimeofday(struct timeval *tp, void *tzp)
{
	int ret;

	/* Timezone obsolete, should be NULL. */
	if (tzp)
		return (-1);

	/* Use the time() to get the seconds. */
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_time),
		  "b" (NULL)
	);

	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		_REENT->_errno = -ret;
		return (-1);
	}
	
	tp->tv_usec = 0;
	tp->tv_sec  = ret;

	return (0);
}

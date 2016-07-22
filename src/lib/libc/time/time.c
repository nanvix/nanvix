/*
 * Copyright(C) 2016-2016 Subhra S. Sarkar <rurtle.coder@gmail.com>
 *              2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
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

/**
 * @brief Get the time elapsed since Epoch in seconds
 * 
 * @param Time structure.
 * 
 * @returns Elapsed time since Epoch in seconds.
 */
time_t time(time_t *tloc)
{
	int ret;

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_time),
		  "b" (tloc)
	);

	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}

	return (ret);
}

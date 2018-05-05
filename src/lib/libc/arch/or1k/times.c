/*
 * Copyright(C) 2011-2018 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2018 Davidson Francis <davidsondfgl@gmail.com>
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
#include <reent.h>

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
	register clock_t elapsed 
		__asm__("r11") = NR_times;
	register unsigned r3
		__asm__("r3") = (unsigned) buffer;
	
	__asm__ volatile (
		"l.sys 1"
		: "=r" (elapsed)
		: "r" (elapsed),
		  "r" (r3)
	);
	
	/* Error. */
	if (elapsed < 0)
	{
		errno = -elapsed;
		_REENT->_errno = -elapsed;
		return (-1);
	}
	
	return (elapsed);
}

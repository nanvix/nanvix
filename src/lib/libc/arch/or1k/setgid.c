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
#include <sys/types.h>
#include <errno.h>
#include <reent.h>

/**
 * @brief Sets user's group ID.
 * 
 * @param gid New user's group ID.
 * 
 * @returns Upon successful completion, 0 is returned. Otherwise, -1 is returned
 *          and errno set to indicate the error.
 */
int setgid(gid_t gid)
{
	register int ret 
		__asm__("r11") = NR_setgid;
	register unsigned r3
		__asm__("r3") = (unsigned) gid;

	__asm__ volatile (
		"l.sys 1"
		: "=r" (ret)
		: "r" (ret),
		  "r" (r3)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		_REENT->_errno = -ret;
		return (-1);
	}
	
	return (ret);
}

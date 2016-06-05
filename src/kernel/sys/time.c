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

#include <sys/types.h>
#include <nanvix/clock.h>
#include <nanvix/mm.h>
#include <errno.h>

/**
 * @brief Returns the value of time in seconds since the Epoch.
 *
 * @param Upon successful completion, the value of time is returned. Otherwise,
 *        (time_t)-1 is returned.
 */
PUBLIC time_t sys_time(time_t *tloc)
{
	time_t ret;
	
	ret = CURRENT_TIME;

	/* Store value in tloc. */
	if (tloc != NULL)
	{
		/* Invalid buffer. */
		if (!chkmem(tloc, sizeof(time_t), MAY_WRITE))
			return (-EFAULT);
	
		*tloc = ret;
	}

	return (ret);
}

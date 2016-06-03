/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 		2016-2016 Subhra S. Sarkar <rurtle.coder@gmail.com>
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

/*
 * @brief Returns the current time since Epoch (00:00:00 UTC 1st Jan, 1970)
 */
PUBLIC time_t sys_time(time_t *tloc)
{
	time_t ret = -1;

	/* Invalid argument check */
	if (tloc == NULL)
		return ret;

	/* current time = time since Epoch to bootup + time since bootup */
	ret = (cmos_gettime() + (ticks / CLOCK_FREQ));
	tloc = &ret;

	return ret;
}

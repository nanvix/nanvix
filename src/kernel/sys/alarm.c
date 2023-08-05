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

#include <nanvix/const.h>
#include <nanvix/clock.h>
#include <nanvix/pm.h>

/*
 * Schedules an alarm signal.
 */
PUBLIC unsigned sys_alarm(unsigned seconds)
{
	unsigned oldalarm;

	oldalarm = curr_proc->alarm;

	/* Schedule alarm. */
	if (seconds > 0)
		curr_proc->alarm = ticks + seconds*CLOCK_FREQ;

	/* Cancel alarm. */
	else
		curr_proc->alarm = 0;

	/* Alarm would ring soon if we had not re-scheduled it. */
	if (oldalarm <= ticks)
		return (0);

	return ((oldalarm - ticks)/CLOCK_FREQ);
}

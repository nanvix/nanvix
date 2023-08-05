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

#include <nanvix/clock.h>
#include <nanvix/const.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <sys/times.h>
#include <errno.h>

/**
 * @brief Gets process and waited-for child process times.
 *
 * @param buffer Timing accounting information.
 *
 * @returns Upon successful completion, the elapsed real time, in clock ticks,
 *          since system start-up time is returned. Upon failure, a negative
 *          error code is returned instead.
 */
PUBLIC clock_t sys_times(struct tms *buffer)
{
	/* Not a valid buffer. */
	if (!chkmem(buffer, sizeof(struct tms), MAY_WRITE))
		return (-EINVAL);

	buffer->tms_utime = curr_proc->utime*CLOCK_FREQ;
	buffer->tms_stime = curr_proc->ktime*CLOCK_FREQ;
	buffer->tms_cutime = curr_proc->cutime*CLOCK_FREQ;
	buffer->tms_cstime = curr_proc->cktime*CLOCK_FREQ;

	return (CURRENT_TIME*CLOCK_FREQ);
}

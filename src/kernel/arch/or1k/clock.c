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
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/pm.h>

/**
 * @brief Clock interrupts since system initialization.
 */
PUBLIC unsigned ticks = 0;

/**
 * @brief Start up time (in seconds).
 */
PUBLIC unsigned startup_time = 0;

/*
 * Handles a timer interrupt.
 */
PRIVATE void do_clock()
{
	ticks++;
	
	if (KERNEL_WAS_RUNNING(curr_proc))
	{
		curr_proc->ktime++;
		return;
	}
	
	curr_proc->utime++;
		
	/* Give up processor time. */
	if (--curr_proc->counter == 0)
		yield();
}

/*
 * Initializes the system's clock.
 */
PUBLIC void clock_init(unsigned freq)
{
	kprintf("dev: initializing clock device driver");
}

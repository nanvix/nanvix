/*
 * Copyright(C) 2011-2018 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2018-2018 Davidson Francis <davidsondfgl@gmail.com>
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

/**
 * @brief Clock interrupts per/sec.
 */
PRIVATE unsigned rate = 0;

/*
 * @brief Setup the clock next event.
 */
PRIVATE void clock_next_event()
{
	unsigned clock, new_clock;
	clock =  mfspr(SPR_TTCR);
	new_clock = clock;
	new_clock += rate;
	new_clock &= SPR_TTMR_TP;

	/* Set counter. */
	mtspr(SPR_TTMR, SPR_TTMR_SR | SPR_TTMR_IE | new_clock);
	mtspr(SPR_TTCR, clock);
}

/*
 * Handles a timer interrupt.
 */
PRIVATE void do_clock()
{
	/* Temporaly disable clock interrupts. */
	mtspr(SPR_TTMR, SPR_TTMR_DI);
	
	ticks++;
	
	if (KERNEL_WAS_RUNNING(curr_proc))
	{
		curr_proc->ktime++;
		clock_next_event();
		return;
	}
	
	curr_proc->utime++;

	/* Setup next event again. */
	clock_next_event();
		
	/* Give up processor time. */
	if (--curr_proc->counter == 0)
		yield();
}

/*
 * Initializes the system's clock.
 */
PUBLIC void clock_init(unsigned freq)
{
	unsigned upr;

	upr = mfspr(SPR_UPR);
	if ( !(upr & SPR_UPR_TTP) )
		kpanic("dev: device without tick timer!");

	kprintf("dev: initializing clock device driver");
	set_hwint(INT_CLOCK, &do_clock);

	/* Clock rate. */
	rate = CPU_CLOCK/freq;

	/* Ensures that the clock is disabled. */
	mtspr(SPR_TTMR, SPR_TTMR_DI);

	/* Setup the clock next event. */
	clock_next_event();

	/* Enable clock. */
	mtspr(SPR_SR, mfspr(SPR_SR) | 2);
}

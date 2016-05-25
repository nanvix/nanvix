/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2016-2016 Subhra S. Sarkar <rurtle.coder@gmail.com>
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

#ifndef TIMER_H_
#define TIMER_H_

	#include <nanvix/const.h>
	
	/* Clock frequency (in hertz). */
	#define CLOCK_FREQ 100
	
	/* Current time. */
	#define CURRENT_TIME (startup_time + ticks/CLOCK_FREQ)

	/* CMOS data structure */
	struct cmos {
		unsigned int sec;		/** Second       */
		unsigned int min;		/** Minutes      */
		unsigned int hour;		/** Hour         */
		unsigned int dom;		/** Day of month */
		unsigned int mon;		/** Month        */
		unsigned int year;		/** Year         */
	};

	/*
 	 * Initializes the CMOS timer
 	 */
	EXTERN void cmos_init(void);

	/*
	 * Initializes the timer interrupt.
	 */
	EXTERN void clock_init(unsigned freq);

	/* Ticks since system initialization. */
	EXTERN unsigned ticks;
	
	/* Time at system startup. */
	EXTERN unsigned startup_time;
	EXTERN struct cmos *start_time;
	
#endif /* TIMER_H_ */

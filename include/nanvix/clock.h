/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
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
	
	/**
	 * @brief Clock interrupt frequency (in Hz)
	 */
	#define CLOCK_FREQ 100
	
	/**
	 * @brief Current time.
	 */
	#define CURRENT_TIME \
		(startup_time + ticks/CLOCK_FREQ)

	/**
	 * @brief CMOS Time structure.
	 */
	struct cmos
	{
		unsigned sec;  /**< Seconds.      */
		unsigned min;  /**< Minutes.      */
		unsigned hour; /**< Hour.         */
		unsigned dom;  /**< Day of Month. */
		unsigned mon;  /**< Month.        */
		unsigned year; /**< Year.         */
	};

 	/* Forward declarations. */
	EXTERN void cmos_init(void);
	EXTERN void clock_init(unsigned);
	EXTERN signed cmos_gettime(void);

	/**
	 * @brief Clock interrupts since system initialization.
	 */
	EXTERN unsigned ticks;
	
	/* Forward declarations. */
	EXTERN unsigned startup_time;
	EXTERN const struct cmos *boot_time;
	
#endif /* TIMER_H_ */

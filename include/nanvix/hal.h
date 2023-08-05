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

/**
 * @file nanvix/hal.h
 *
 * @brief Hardware Abstraction Layer
 */

#ifndef NANVIX_HAL_H_
#define NANVIX_HAL_H_

#ifndef _ASM_FILE_

	#include <i386/i386.h>
	#include <i386/int.h>

	#include <nanvix/const.h>
	#include <nanvix/pm.h>
	#include <stdlib.h>

	/* Forward definitions. */
	struct process;

	/**
	 * @name Interrupt Priority Levels
	 */
	/**@{*/
	#define INT_LVL_0 0 /**< Level 0: all hardware interrupts disabled. */
	#define INT_LVL_1 1 /**< Level 1: clock interrupts enabled.         */
	#define INT_LVL_2 2 /**< Level 2: disk interrupts enabled.          */
	#define INT_LVL_3 3 /**< Level 3: network interrupts enabled.       */
	#define INT_LVL_4 4 /**< Level 4: terminal interrupts enabled.      */
	#define INT_LVL_5 5 /**< Level 5: all hardware interrupts enabled.  */
	/**@}*/

	/**
	 * @name Processor Control Functions
	 */
	/**@{*/
	EXTERN int set_hwint(int, void (*)(void));
	EXTERN void enable_interrupts(void);
	EXTERN void disable_interrupts(void);
	EXTERN void halt(void);
	EXTERN void processor_drop(unsigned);
	EXTERN unsigned processor_raise(unsigned);
	EXTERN void processor_reload(void);
	EXTERN void setup(void);
	EXTERN void user_mode(addr_t, addr_t);
	EXTERN void switch_to(struct process *);
	EXTERN unsigned irq_lvl(unsigned);
	/**@}*/

	/**
	 * @name I/O Functions
	 */
	/**@{*/
	EXTERN void iowait(void);
	EXTERN void outputb(word_t, byte_t);
	EXTERN void outputw(word_t, word_t);
	EXTERN byte_t inputb(word_t);
	EXTERN word_t inputw(word_t);
	/**@}*/

	/**
	 * @name Memory Functions
	 */
	/**@{*/
	EXTERN void physcpy(addr_t, addr_t, size_t);
	/**@}*/

#endif /* _ASM_FILE_ */

#endif /* NANVIX_HAL_H_ */

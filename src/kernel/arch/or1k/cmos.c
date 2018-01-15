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

#include <nanvix/hal.h>
#include <nanvix/clock.h>

/**
 * @brief Current millennium.
 */
#define CURR_MILLENNIUM 2000

/**
 * @brief CMOS Time Structure.
 */
PRIVATE struct
{
	unsigned sec;  /**< Seconds.      */
	unsigned min;  /**< Minutes.      */
	unsigned hour; /**< Hour.         */
	unsigned dom;  /**< Day of Month. */
	unsigned mon;  /**< Month.        */
	unsigned year; /**< Year.         */
} boot_time;

/**
 * @brief Read CMOS device.
 * 
 * @param addr Target address.
 */
PRIVATE unsigned cmos_read(unsigned addr)
{
}

/**
 * @brief Returns time in seconds since Epoch (00:00:00 UTC, 1st Jan,1970) till
 *        bootup.
 *
 * @NOTE  Since register 0x09 gives only last 2 digits of the year, it's our
 *        responsibility to add it with right offset.
 *
 */
PRIVATE signed cmos_gettime(void)
{
	return 0;
}

/**
 * @brief Initializes the CMOS device.
 */
PUBLIC void cmos_init(void)
{
}

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
 * @brief Bootup time.
 */
PUBLIC const struct cmos *boot_time;

/**
 * @brief Private _boot_time.
 */
PRIVATE struct cmos _boot_time;

/**
 * @brief Read CMOS device.
 * 
 * @param addr Target address.
 */
PRIVATE unsigned int cmos_read(unsigned addr)
{
	/*
	 * Disable NMI at the highest order bit.
	 */
	outputb(0x70, 0x80 | addr);
	
	return (inputb(0x71));
}

/**
 * @brief Initializes the CMOS device.
 */
PUBLIC void cmos_init(void)
{
	unsigned int registerB;

	/*
	 * Repeatedly read the register values and store them into
	 * global cmos structure till you find duplicate values.
	 */
	do
	{
		_boot_time.sec  = cmos_read(0x00);
		_boot_time.min  = cmos_read(0x02);
		_boot_time.hour = cmos_read(0x04);
		_boot_time.dom  = cmos_read(0x07);
		_boot_time.mon  = cmos_read(0x08);
		_boot_time.year = cmos_read(0x09);
	} while (_boot_time.sec != cmos_read(0));

	/* Read output format information from CMOS registers. */
	registerB = cmos_read(0x0B);

	/* If output is in BCD format, convert it to binary. */
	if (!(registerB & 0x04))
	{
		_boot_time.sec  = (_boot_time.sec & 0x0f) + ((_boot_time.sec/16)*10);
		_boot_time.min  = (_boot_time.min & 0x0f) + ((_boot_time.min/16)*10);
		_boot_time.hour = ((_boot_time.hour & 0x0f) + (((_boot_time.hour & 0x70)/16)*10))
		               | (_boot_time.hour & 0x80);
		_boot_time.dom  = (_boot_time.dom & 0x0f) + ((_boot_time.dom/16)*10);
		_boot_time.mon  = (_boot_time.mon & 0x0f) + ((_boot_time.mon/16)*10);
		_boot_time.year = (_boot_time.year & 0x0f) + ((_boot_time.year/16)*10);
	}

	/* Convert 12 hr clock to 24 hr clock if necessary. */
	if (!(registerB & 0x02) && (_boot_time.hour & 0x80))
		_boot_time.hour = ((_boot_time.hour & 0x7f) + 12)%24;
	
	boot_time = &_boot_time;
}

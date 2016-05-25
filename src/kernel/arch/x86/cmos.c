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
 * @brief CMOS time structure.
 */
PRIVATE struct cmos cmos_tm;

/**
 * @brief Start time.
 */
PUBLIC struct cmos *start_time = &cmos_tm;

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
		cmos_tm.sec  = cmos_read(0x00);
		cmos_tm.min  = cmos_read(0x02);
		cmos_tm.hour = cmos_read(0x04);
		cmos_tm.dom  = cmos_read(0x07);
		cmos_tm.mon  = cmos_read(0x08);
		cmos_tm.year = cmos_read(0x09);
	} while (cmos_tm.sec != cmos_read(0));

	/* Read output format information from CMOS registers */
	registerB = cmos_read(0x0B);

	/* If output is in BCD format, convert it to binary */
	if (!(registerB & 0x04))
	{
		cmos_tm.sec  = (cmos_tm.sec & 0x0f) + ((cmos_tm.sec/16)*10);
		cmos_tm.min  = (cmos_tm.min & 0x0f) + ((cmos_tm.min/16)*10);
		cmos_tm.hour = ((cmos_tm.hour & 0x0f) + (((cmos_tm.hour & 0x70)/16)*10))
		               | (cmos_tm.hour & 0x80);
		cmos_tm.dom  = (cmos_tm.dom & 0x0f) + ((cmos_tm.dom/16)*10);
		cmos_tm.mon  = (cmos_tm.mon & 0x0f) + ((cmos_tm.mon/16)*10);
		cmos_tm.year = (cmos_tm.year & 0x0f) + ((cmos_tm.year/16)*10);
	}

	/* Convert 12 hr clock to 24 hr clock if necessary */
	if (!(registerB & 0x02) && (cmos_tm.hour & 0x80))
		cmos_tm.hour = ((cmos_tm.hour & 0x7f) + 12)%24;
}

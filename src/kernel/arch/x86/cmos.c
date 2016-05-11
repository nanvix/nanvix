/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 				2016-2016 Subhra S. Sarkar <rurtle.coder@gmail.com>
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

#define BCD_TO_BIN(value)	((value)=((value)&15)+((value)>>4)*10)

static unsigned int read_cmos_reg(unsigned int addr)
{
	/* Don't forget to disable NMI at the highest order bit */
	outputb(0x80 | addr, 0x70);
	return inputb(0x71);
}

PUBLIC void cmos_init(void)
{
	struct cmos cmos_tm;
	unsigned int registerB;		/* Controls format of RTC bytes */
	/* Well, ideally we should add checks for all cmos attributes */
 	/* to ensure we don't get the same value twice                */
	/* NOTE: Need to discuss this with the group                  */
	do
	{
		cmos_tm.sec  = read_cmos_reg(0x00);
		cmos_tm.min  = read_cmos_reg(0x02);
		cmos_tm.hour = read_cmos_reg(0x04);
		cmos_tm.dom  = read_cmos_reg(0x07);
		cmos_tm.mon  = read_cmos_reg(0x08);
		cmos_tm.year = read_cmos_reg(0x09);
	} while (cmos_tm.sec != read_cmos_reg(0));

	/* Read output format information from CMOS registers */
	registerB = read_cmos_reg(0x0B);

	/* If output is in BCD format, convert it to binary */
	if (!(registerB & 0x04))
	{
		cmos_tm.sec  = (cmos_tm.sec & 0x0F) + ((cmos_tm.sec / 16) * 10);
		cmos_tm.min  = (cmos_tm.min & 0x0F) + ((cmos_tm.min / 16) * 10);
		cmos_tm.hour = ( (cmos_tm.hour & 0x0F) + \
						(((cmos_tm.hour & 0x70) / 16) * 10) ) | \
						(cmos_tm.hour & 0x80);
		cmos_tm.dom  = (cmos_tm.dom & 0x0F) + ((cmos_tm.dom / 16) * 10);
		cmos_tm.mon  = (cmos_tm.mon & 0x0F) + ((cmos_tm.mon / 16) * 10);
		cmos_tm.year = (cmos_tm.year & 0x0F) + ((cmos_tm.year / 16) * 10);
	}

	/* Convert 12 hr clock to 24 hr clock if necessary */
	if (!(registerB & 0x02) && (cmos_tm.hour & 0x80))
		cmos_tm.hour = ((cmos_tm.hour & 0x7F) + 12) % 24;
}

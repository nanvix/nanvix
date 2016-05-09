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
	/* Well, ideally we should add checks for all cmos attributes */
 	/* to ensure we don't get the same value twice                */
	/* NOTE: Need to discuss this with the group                  */
	do
	{
		cmos_tm.cmos_sec  = read_cmos_reg(0x00);
		cmos_tm.cmos_min  = read_cmos_reg(0x02);
		cmos_tm.cmos_hour = read_cmos_reg(0x04);
		cmos_tm.cmos_dom  = read_cmos_reg(0x07);
		cmos_tm.cmos_mon  = read_cmos_reg(0x08);
		cmos_tm.cmos_year = read_cmos_reg(0x09);
	} while (cmos_tm.cmos_sec != read_cmos_reg(0));

	/* Copied from linux-0.01. Thanks Mr. Torvalds! */
	BCD_TO_BIN(cmos_tm.cmos_sec);
	BCD_TO_BIN(cmos_tm.cmos_min);
	BCD_TO_BIN(cmos_tm.cmos_hour);
	BCD_TO_BIN(cmos_tm.cmos_dom);
	BCD_TO_BIN(cmos_tm.cmos_mon);
	BCD_TO_BIN(cmos_tm.cmos_year);
}

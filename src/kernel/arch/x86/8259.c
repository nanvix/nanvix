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

#include <i386/i386.h>
#include <nanvix/const.h>
#include <nanvix/hal.h>
#include <stdint.h>

/*============================================================================*
 *                               pic_mask()                                   *
 *============================================================================*/

/*
 * Sets interrupt mask.
 */
PUBLIC void pic_mask(uint16_t mask)
{
	outputb(PIC_DATA_MASTER, mask & 0xff);
	outputb(PIC_DATA_SLAVE, mask >> 8);
}

/*============================================================================*
 *                               pic_setup()                                  *
 *============================================================================*/

/*
 * Setups the programmable interrupt controller
 */
PUBLIC void pic_setup(uint8_t offset1, uint8_t offset2)
{
	/*
	 * Starts initialization sequence
	 * in cascade mode.
	 */
	outputb(PIC_CTRL_MASTER, 0x11);
	iowait();
	outputb(PIC_CTRL_SLAVE, 0x11);
	iowait();

	/* Send new vector offset. */
	outputb(PIC_DATA_MASTER, offset1);
	iowait();
	outputb(PIC_DATA_SLAVE, offset2);
	iowait();

	/*
	 * Tell the master that there is a slave
	 * PIC hired up at IRQ line 2 and tell
	 * the slave PIC that it is the second PIC.
	 */
	outputb(PIC_DATA_MASTER, 0x04);
	iowait();
	outputb(PIC_DATA_SLAVE, 0x02);
	iowait();

	/* Set 8086 mode. */
	outputb(PIC_DATA_MASTER, 0x01);
	iowait();
	outputb(PIC_DATA_SLAVE, 0x01);
	iowait();

	/* Clears interrupt mask. */
	pic_mask(0x0000);
}

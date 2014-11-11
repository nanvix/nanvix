/*
 * Copyright(C) 2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <asm/io.h>
#include <i386/8259.h>
#include <nanvix/const.h>
#include <stdint.h>

/**
 * @brief Sets interrupt mask.
 *
 * @details Sets interrupt, preventing masked IRQs to be fired.
 *
 * @param mask Interrupt mask.
 */
PUBLIC void pic_mask(uint16_t mask)
{
	outputb(PIC_DATA_MASTER, mask & 0xff);
	outputb(PIC_DATA_SLAVE, mask >> 8);
}

/**
 * @brief Sets up interrupts.
 *
 * @details Sets up interrupts by remapping PIC interrupts.
 */
PUBLIC void pic_remap(void)
{
	outputb(PIC_CTRL_MASTER, 0x11);
	outputb(PIC_CTRL_SLAVE, 0x11);
	outputb(PIC_DATA_MASTER, 0x20);
	outputb(PIC_DATA_SLAVE, 0x28);
	outputb(PIC_DATA_MASTER, 0x04);
	outputb(PIC_DATA_SLAVE, 0x02);
	outputb(PIC_DATA_MASTER, 0x01);
	outputb(PIC_DATA_SLAVE, 0x01);
	outputb(PIC_DATA_MASTER, 0x00);
	outputb(PIC_DATA_SLAVE, 0x00);
}

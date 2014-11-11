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

#include <i386/8259.h>
#include <nanvix/const.h>
#include <nanvix/hal.h>

/**
 * @brief Interrupt priority levels.
 */
PRIVATE const unsigned int_lvls[16] = {
	INT_LVL_0, /* Programmable interrupt timer.         */
	INT_LVL_3, /* Keyboard.                             */
	INT_LVL_3, /* COM2.                                 */
	INT_LVL_3, /* COM1.                                 */
	INT_LVL_3, /* LPT2.                                 */
	INT_LVL_1, /* Floppy disk.                          */
	INT_LVL_3, /* LPT1.                                 */
	INT_LVL_0, /* CMOS real-time clock.                 */
	INT_LVL_2, /* Legacy SCSI or NIC.                   */
	INT_LVL_2, /* Legacy SCSI or NIC.                   */
	INT_LVL_2, /* Legacy SCSI or NIC.                   */
	INT_LVL_3, /* PS2 mouse.                            */
	INT_LVL_4, /* FPU, co-processor or inter-processor. */
	INT_LVL_1, /* Primary ATA hard disk.                */
	INT_LVL_1  /* Secondary ATA hard disk.              */
};

/*
 * Returns the interrupt priority level that is associated to an IRQ.
 */
PUBLIC unsigned irq_lvl(unsigned irq)
{
	return (int_lvls[irq]);
}

/**
 * @brief Interrupt masks table.
 */
PRIVATE const uint16_t int_masks[6] = {
	0xfffb, /* Clock priority level.        */
	0xfefa, /* Disk priority level.         */
	0x3eba, /* Network priority level.      */
	0x30ba, /* Terminal priority level.     */
	0x2000, /* Co-processor priority level. */
	0x0000  /* Lowest priority level.       */
};

PRIVATE unsigned stack_lvl[6] = { 5, 5, 5, 5, 5, 5 };
PRIVATE unsigned stack_ptr = 0;

/*
 * Raises processor execution level.
 */
PUBLIC void processor_raise(unsigned lvl)
{
	stack_lvl[++stack_ptr] = lvl;

	pic_mask(int_masks[lvl]);
}

/*
 * Drops processor execution level.
 */
PUBLIC void processor_drop(void)
{
	unsigned lvl;

	lvl = stack_lvl[--stack_ptr];

	pic_mask(int_masks[lvl]);
}

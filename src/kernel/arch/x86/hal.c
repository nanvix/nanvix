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

#include <i386/i386.h>
#include <nanvix/const.h>
#include <nanvix/hal.h>
#include <stdint.h>

/*============================================================================*
 *                               irq_lvl()                                    *
 *============================================================================*/

/*
 * Interrupt priority levels.
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

/*============================================================================*
 *                              processor_*()                                 *
 *============================================================================*/

/*
 * Interrupt masks table.
 */
PRIVATE const uint16_t int_masks[6] = {
	0xfffb, /* Level 0: all hardware interrupts disabled. */
	0xfefa, /* Level 1: clock interrupts enabled.         */
	0x3eba, /* Level 2: disk interrupts enabled.          */
	0x30ba, /* Level 3: network interrupts enabled        */
	0x2000, /* Level 4: terminal interrupts enabled.      */
	0x0000  /* Level 5: all hardware interrupts enabled.  */
};

/*
 * Interrupt stack level.
 */
PRIVATE struct
{
	unsigned data[6]; /* Data.          */
	unsigned ptr;     /* Stack pointer. */
} stack = {
	{ 5, 5, 5, 5, 5, 5 },
	0
};

/*
 * Raises processor execution level.
 */
PUBLIC void processor_raise(unsigned lvl)
{
	stack.data[++stack.ptr] = lvl;

	pic_mask(int_masks[lvl]);
}

/*
 * Drops processor execution level.
 */
PUBLIC void processor_drop(void)
{
	unsigned lvl;

	lvl = stack.data[--stack.ptr];

	pic_mask(int_masks[lvl]);
}

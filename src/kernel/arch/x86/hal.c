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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <i386/8259.h>
#include <nanvix/const.h>

/* Interrupt priority levels. */
#define INT_LVL_CLOCK    0
#define INT_LVL_DISK     1
#define INT_LVL_NETWORK  2
#define INT_LVL_TERMINAL 3
#define INT_LVL_COPROC   4
#define INT_LVL_LOWEST   5

/**
 * @brief Interrupt priority levels.
 */
PRIVATE const unsigned int_lvls[16] = {
	INT_LVL_CLOCK,    /* Programmable interrupt timer.         */
	INT_LVL_TERMINAL, /* Keyboard.                             */
	INT_LVL_TERMINAL, /* COM2.                                 */
	INT_LVL_TERMINAL, /* COM1.                                 */
	INT_LVL_TERMINAL, /* LPT2.                                 */
	INT_LVL_DISK,     /* Floppy disk.                          */
	INT_LVL_TERMINAL, /* LPT1.                                 */
	INT_LVL_CLOCK,    /* CMOS real-time clock.                 */
	INT_LVL_NETWORK,  /* Legacy SCSI or NIC.                   */
	INT_LVL_NETWORK,  /* Legacy SCSI or NIC.                   */
	INT_LVL_NETWORK,  /* Legacy SCSI or NIC.                   */
	INT_LVL_TERMINAL, /* PS2 mouse.                            */
	INT_LVL_COPROC,   /* FPU, co-processor or inter-processor. */
	INT_LVL_DISK,     /* Primary ATA hard disk.                */
	INT_LVL_DISK      /* Secondary ATA hard disk.              */
};

/**
 * @brief Returns the interrupt priority level of an IRQ.
 * 
 * @details Returns the interrupt level that is associated to an IRQ.
 * 
 * @param irq IRQ to be queried.
 * 
 * @returns The interrupt priority level that is associated to the IRQ.
 */
PUBLIC unsigned int_lvl(unsigned irq)
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

/**
 * @brief Raises processor execution level
 * 
 * @details Raises processor execution level to mask all interrupts that have a
 *          priority level bellow or equal to the given threshold.
 * 
 * @param lvl Level to raise processor.
 */
PUBLIC void processor_raise(unsigned lvl)
{
	stack_lvl[++stack_ptr] = lvl;
	
	pic_mask(int_masks[lvl]);
}

/**
 * @brief Raises processor execution level
 * 
 * @details Raises processor execution level to mask all interrupts that have a
 *          priority level bellow or equal to the given threshold.
 * 
 * @param lvl Level to raise processor.
 */
PUBLIC void processor_drop(void)
{
	unsigned lvl;
	
	lvl = stack_lvl[--stack_ptr];
	
	pic_mask(int_masks[lvl]);
}

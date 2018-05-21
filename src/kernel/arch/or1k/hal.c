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

#include <or1k/or1k.h>
#include <nanvix/const.h>
#include <nanvix/hal.h>
#include <nanvix/pm.h>
#include <stdint.h>

/*============================================================================*
 *                               irq_lvl()                                    *
 *============================================================================*/

/*
 * Interrupt priority levels.
 */
PRIVATE const unsigned int_lvls[16] = {
	INT_LVL_3, /* Keyboard.                             */
	INT_LVL_3, /* COM2.                                 */
	INT_LVL_3, /* COM1.                                 */
	INT_LVL_3, /* LPT2.                                 */
	INT_LVL_0, /* Programmable interrupt timer.         */
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
PRIVATE const uint32_t int_masks[6] = {
	0x00000000, /* Level 0: all hardware interrupts disabled. */
	0x00000000, /* Level 1: clock interrupts enabled.           */
	0x00000000, /* Level 2: disk interrupts enabled.            */
	0x00000000, /* Level 3: network interrupts enabled          */
	0x00000004, /* Level 4: terminal interrupts enabled.        */
	0x00000004  /* Level 5: 'all' hardware interrupts enabled.  */
};

/**
 * @brief Raises processor execution level.
 * 
 * @note This function must be called in an interrupt-safe environment.
 */
PUBLIC unsigned processor_raise(unsigned irq)
{
	unsigned old_irqlvl;
	unsigned irqlvl;
	
	irqlvl = irq_lvl(irq);
	old_irqlvl = curr_proc->irqlvl;

	/* Mask timer if needed. */
	if (irqlvl == INT_LVL_0)
	{
		/* Timer ACK. */
		mtspr(SPR_TTMR, mfspr(SPR_TTMR) & ~SPR_TTMR_IP);
		mtspr(SPR_SR, mfspr(SPR_SR) & ~SPR_SR_TEE);
	}

	/* Mask other interrupts. */
	pic_mask(int_masks[irqlvl]);

	/* Ack interrupt. */
	pic_ack(irq);

	return (old_irqlvl);
}

/**
 * @brief Drops processor execution level.
 * 
 * @note This function must be called in an interrupt-safe environment.
 */
PUBLIC void processor_drop(unsigned irqlvl)
{
	curr_proc->irqlvl = irqlvl;
	
	if (irqlvl > INT_LVL_0)
		mtspr(SPR_SR, mfspr(SPR_SR) | SPR_SR_TEE);

	/* Previous state. */
	pic_mask(int_masks[irqlvl]);
}

/**
 * @brief Reloads the processor execution level.
 * 
 * @note This function must be called in an interrupt-safe environment.
 */
PUBLIC void processor_reload(void)
{
	if (curr_proc->irqlvl > INT_LVL_0)
	{
		mtspr(SPR_SR, mfspr(SPR_SR) | SPR_SR_TEE);
		mtspr(SPR_TTMR, mfspr(SPR_TTMR) | SPR_TTMR_IE);
	}

	/* Previous state. */
	pic_mask(int_masks[curr_proc->irqlvl]);
}

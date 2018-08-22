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
#include <nanvix/smp.h>
#include <stdint.h>

/*============================================================================*
 *                               irq_lvl()                                    *
 *============================================================================*/

/*
 * Interrupt priority levels.
 */
PRIVATE const unsigned int_lvls[16] = {
	INT_LVL_0, /* Timer.        */
	INT_LVL_1, /* OMPIC.        */
	INT_LVL_2, /* Serial port.  */
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
	0x00000000, /* Level 0: all hardware interrupts disabled.        */
	0x00000001, /* Level 1: clock interrupts enabled.                */
	0x00000002, /* Level 2: clock, ompic interrupts enabled.         */
	0x00000006, /* Level 3: clock, ompic, serial interrupts enabled. */
	0x00000006, /* Level 4-5: 'all' hardware interrupts enabled.     */
	0x00000006
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
	old_irqlvl = cpus[smp_get_coreid()].curr_thread->irqlvl;

	/* Timer ACK as soon as possible. */
	if (irqlvl == INT_LVL_0)
		mtspr(SPR_TTMR, mfspr(SPR_TTMR) & ~SPR_TTMR_IP);

	/* Ack interrupt. */
	pic_ack(irq);

	/* Mask other interrupts. */
	pic_mask(int_masks[irqlvl]);

	return (old_irqlvl);
}

/**
 * @brief Drops processor execution level.
 * 
 * @note This function must be called in an interrupt-safe environment.
 */
PUBLIC void processor_drop(unsigned irqlvl)
{
	cpus[smp_get_coreid()].curr_thread->irqlvl = irqlvl;
	
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
	if (cpus[smp_get_coreid()].curr_thread->irqlvl > INT_LVL_0)
	{
		mtspr(SPR_SR, mfspr(SPR_SR) | SPR_SR_TEE);
		mtspr(SPR_TTMR, mfspr(SPR_TTMR) | SPR_TTMR_IE);
	}

	/* Previous state. */
	pic_mask(int_masks[cpus[smp_get_coreid()].curr_thread->irqlvl]);
}

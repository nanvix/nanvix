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
#include <stdint.h>

/*============================================================================*
 *                               pic_mask()                                   *
 *============================================================================*/
 
/*
 * Sets interrupt mask.
 */
PUBLIC void pic_mask(uint32_t mask)
{
	/* Mask external interrupt. */
	mtspr(SPR_PICMR, mask);
}

/*============================================================================*
 *                               pic_mask()                                   *
 *============================================================================*/
 
/*
 * Sets interrupt mask.
 */
PUBLIC void pic_ack(uint32_t irq)
{
	/* ACK interrupt. */
	mtspr(SPR_PICSR, (1UL << irq));
}

/*============================================================================*
 *                               pic_setup()                                  *
 *============================================================================*/

/*
 * Setups the programmable interrupt controller
 */
PUBLIC void pic_setup(void)
{
	/* Unmask all IRQs. */
	pic_mask(0xFF);
	mtspr(SPR_PICSR, 0);
}

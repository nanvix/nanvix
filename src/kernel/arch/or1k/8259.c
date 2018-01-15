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
PUBLIC void pic_mask(uint16_t mask)
{
}

/*============================================================================*
 *                               pic_setup()                                  *
 *============================================================================*/

/*
 * Setups the programmable interrupt controller
 */
PUBLIC void pic_setup(uint8_t offset1, uint8_t offset2)
{
}

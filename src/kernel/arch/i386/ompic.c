/*
 * Copyright(C) 2018-2018 Davidson Francis <davidsondfgl@gmail.com>
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
#include <or1k/ompic.h>
#include <nanvix/const.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/smp.h>
#include <stdint.h>

/*
 * @brief Reds from a specified OMPIC register.
 * @param reg Target register.
 * @return Register value.
 */
PUBLIC uint32_t ompic_readreg(uint32_t reg)
{
	((void)reg);
	return (0);
}
 
/*
 * @brief Writes into a specified OMPIC register.
 * @param reg Target register.
 * @param data Data to be written.
 */
PUBLIC void ompic_writereg(uint32_t reg, uint32_t data)
{
	((void)reg);
	((void)data);
}

/*
 * Sends an Inter-processor Interrupt.
 * @param dstcore Destination core to be sent the message.
 * @param data Data to be sent.
 */
PUBLIC void ompic_send_ipi(uint32_t dstcore, uint16_t data)
{
	((void)dstcore);
	((void)data);
}

/*
 * Handles to Inter-processor Interrupt here.
 */
PUBLIC void ompic_handle_ipi(void)
{
}

/*
 * Setup the OMPIC.
 */
PUBLIC void ompic_init(void)
{
}

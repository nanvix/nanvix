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

#include <or1k/or1k.h>
#include <or1k/ompic.h>
#include <nanvix/const.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/smp.h>
#include <stdint.h>

#define INPUTW(addr) *((volatile uint32_t *) (addr)) 
#define OUTPUTW(addr, value) ((INPUTW(addr)) = (value))

/*
 * @brief Reds from a specified OMPIC register.
 * @param reg Target register.
 * @return Register value.
 */
PUBLIC uint32_t ompic_readreg(uint32_t reg)
{
	return INPUTW(reg);
}
 
/*
 * @brief Writes into a specified OMPIC register.
 * @param reg Target register.
 * @param data Data to be written.
 */
PUBLIC void ompic_writereg(uint32_t reg, uint32_t data)
{
	OUTPUTW(reg, data);
}

/*
 * Sends an Inter-processor Interrupt.
 * @param dstcore Destination core to be sent the message.
 * @param data Data to be sent.
 */
PUBLIC void ompic_send_ipi(uint32_t dstcore, uint16_t data)
{
	ompic_writereg(OMPIC_CTRL(smp_get_coreid()), OMPIC_CTRL_IRQ_GEN |
		OMPIC_CTRL_DST(dstcore)| OMPIC_DATA(data));
}

/*
 * Handles to Inter-processor Interrupt here.
 */
PUBLIC void ompic_handle_ipi(void)
{
	unsigned cpu = smp_get_coreid();

	/* ACK IPI. */
	ompic_writereg(OMPIC_CTRL(cpu), OMPIC_CTRL_IRQ_ACK);
}

/*
 * Setup the OMPIC.
 */
PUBLIC void ompic_init(void)
{

}

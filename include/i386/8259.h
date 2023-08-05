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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

/*
 * File: arch/i386/8259.h
 *
 * 8259 Programmable Interrupt Controller
 *
 * Description:
 *
 *     The 8259 Programmable Interrupt Controller (PIC) is a core component of
 *     x86 processors: it manages hardware interrupts, firing the appropriate
 *     system interrupts. Indeed, without this component, the x86 architecture
 *     would not be an interrupt driven architecture.
 */

#ifndef PIC_H_
#define PIC_H_

	#include <nanvix/const.h>
	#include <stdint.h>

	/*
	 * Constants: Master PIC Registers
	 *
	 * PIC_CTRL_MASTER - Control register.
	 * PIC_DATA_MASTER - Data register.
	 */
	#define PIC_CTRL_MASTER 0x20
	#define PIC_DATA_MASTER 0x21

	/*
	 * Constants: Slave PIC Registers
	 *
	 * PIC_CTRL_SLAVE - Control register.
	 * PIC_DATA_SLAVE - Data register.
	 */
	#define PIC_CTRL_SLAVE 0xa0
	#define PIC_DATA_SLAVE 0xa1

#ifndef _ASM_FILE_

	/*
	 * Function: pic_mask
	 *
	 * Sets interrupt mask.
	 *
	 * Parameters:
	 *
	 *     mask - Interrupt mask to be set.
	 *
	 * Description:
	 *
	 *     The <pic_mask> function sets the interrupt mask to _mask_, thus
	 *     preventing related interrupt requested to be fired.
	 */
	EXTERN void pic_mask(uint16_t mask);

	/*
	 * Function: pic_setup
	 *
	 * Setups the programmable interrupt controller.
	 *
	 * Parameters:
	 *
	 *     offset1 - Vector offset for master PIC.
	 *     offset2 - Vector offset for slave PIC.
	 *
	 * Description:
	 *
	 *     The <pic_setup> function setups the PIC by effectively remapping
	 *     interrupt vectors. This is mandatory when operating in protected
	 *     mode, since the default hardware interrupt vectors conflicts with
	 *     CPU exception vectors.
	 */
	EXTERN void pic_setup(uint8_t offset1, uint8_t offset2);

#endif /* _ASM_FILE_ */

#endif /* PIC */


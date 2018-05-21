/*
 * Copyright(C) 2011-2018 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2018-2018 Davidson Francis <davidsondfgl@gmail.com>
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
 * File: or1k/8259.h
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
	EXTERN void pic_mask(uint32_t mask);

	/*
	 * Function: pic_ack
	 * 
	 * Acknowledges an interrupt.
	 * 
	 * Parameters:
	 * 
	 *     irq - Interrupt IRQ to be acknowledge.
	 * 
	 * Description:
	 * 
	 *     The <pic_ack> function acknowledges the IRQ, preventing to occur the
	 *     same interrupt again.
	 */
	EXTERN void pic_ack(uint32_t irq);
	
	/*
	 * Function: pic_setup
	 * 
	 * Setups the programmable interrupt controller.
	 * 
	 * Description:
	 * 
	 *     The <pic_setup> function setups the PIC by unmasking all IRQs.
	 */
	EXTERN void pic_setup(void);

#endif /* _ASM_FILE_ */

#endif /* PIC */


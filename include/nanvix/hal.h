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

/*
 * File: nanvix/hal.h
 * 
 * Hardware Abstraction Layer
 * 
 * Description:
 * 
 *     The Hardware Abstraction Layer (HAL) provides an abstraction of the 
 *     underlying hardware, presenting a uniform interface to the remainder
 *     portions of the operating system. Thereby, it may operate different
 *     hardware in a transparent way.
 */

#ifndef HAL_H_
#define HAL_H_
	
	#include <nanvix/const.h>
	#include <nanvix/pm.h>
	#include <stdlib.h>
 
/*============================================================================*
 * Section: Processor                                                         *
 *============================================================================*/
	
	/*
	 * Constants: Interrupt Priority Levels
	 * 
	 * INT_LVL_0 - Level 0: all hardware interrupts disabled.
	 * INT_LVL_1 - Level 1: clock interrupts enabled.
	 * INT_LVL_2 - Level 2: disk interrupts enabled.
	 * INT_LVL_3 - Level 3: network interrupts enabled
	 * INT_LVL_4 - Level 4: terminal interrupts enabled.
	 * INT_LVL_5 - Level 5: all hardware interrupts enabled.
	 */
	#define INT_LVL_0 0
	#define INT_LVL_1 1
	#define INT_LVL_2 2
	#define INT_LVL_3 3
	#define INT_LVL_4 4
	#define INT_LVL_5 5
	
	/* 
	 * Function: disable_interrupts
	 * 
	 * Disables all hardware interrupts.
	 * 
	 * Description:
	 * 
	 *     The <disable_interrupts> function disables all hardware interrupts
	 *     by effectively clearing the CPU interrupt flag.
	 * 
	 * See also:
	 * 
	 *     <enable_interrupts>
	 */
	EXTERN void disable_interrupts(void);

	/* 
	 * Function: enable_interrupts
	 * 
	 * Enables all hardware interrupts.
	 * 
	 * Description:
	 * 
	 *     The <enable_interrupts> function enables all hardware interrupts
	 *     by effectively setting the CPU interrupt flag.
	 * 
	 * See also:
	 *     <disable_interrupts>
	 */
	EXTERN void enable_interrupts(void);
	
	/*
	 * Function: halt
	 * 
	 * Halts the processor.
	 * 
	 * Description:
	 * 
	 *     The <halt> function halts the processor by first disabling all
	 *     hardware interrupts and then entering in a tight and infinite loop.
	 * 
	 * See also:
	 * 
	 *     <disable_interrupts>
	 */
	EXTERN void halt(void);
	
	/*
	 * Function: irq_lvl
	 * 
	 * Returns the interrupt priority level that is associated to an IRQ.
	 * 
	 * Parameters:
	 * 
	 *     irq - IRQ to be queried.
	 * 
	 * Description:
	 * 
	 *     The <irq_lvl> returns the interrupt level that is associated to the
	 *     IRQ which number is _irq_.
	 * 
	 * Return:
	 * 
	 *     The interrupt level that is associated to the requested IRQ is 
	 *     returned.
	 * 
	 * See also:
	 * 
	 *     <processor_drop>, <processor_raise>
	 */
	EXTERN unsigned irq_lvl(unsigned irq);
	
	/*
	 * Function: processor_drop
	 * 
	 * Drops processor execution level.
	 * 
	 * Description:
	 * 
	 *     The <processor_drop> function drops the processor execution level, so
	 *     that interrupts in the current level are re-enabled.
	 * 
	 * Notes:
	 * 
	 *     - This function must be called in an interrupt-safe environment.
	 * 
	 * See also:
	 * 
	 *     <enable_interrupts>, <disable_interrupts>
	 */
	EXTERN void processor_drop(void);
	
	/*
	 * Function: processor_raise
	 * 
	 * Raises processor execution level.
	 * 
	 * Parameters:
	 * 
	 *     lvl - Level to raise processor.
	 * 
	 * Description:
	 * 
	 *     The <processor_raise> function raises processor execution level, so
	 *     that interrupts in the _lvl_ level are disabled.
	 * 
	 * Notes:
	 * 
	 *     - This function must be called in an interrupt-safe environment.
	 * 
	 * See also:
	 * 
	 *     <enable_interrupts>, <disable_interrupts>,
	 *     <Interrupt Priority Levels>
	 */
	EXTERN void processor_raise(unsigned lvl);
	
	/*
	 * Function: setup
	 * 
	 * Sets up machine.
	 * 
	 * Description:
	 * 
	 *     The <setup> function sets up platform-specific structures, so as the
	 *     system can correctly operates.
	 */
	EXTERN void setup(void);

	/*
	 * Function: switch_to
	 * 
	 * Switches execution to a process.
	 * 
	 * Parameters:
	 * 
	 *     proc - Process to switch to.
	 * 
	 * Description:
	 * 
	 *     The <switch_to> function switches executing from the calling 
	 *     process to the process pointed to by _proc_. To do so, first the
	 *     context of the calling process is stored, and then the context of the
	 *     _proc_ process is restored.
	 * 
	 * Notes:
	 * 
	 *     - This function must be called in a interrupt-safe environment.
	 * 
	 * See also:
	 * 
	 *     <user_mode>, <Processor Functions>
	 */
	EXTERN void switch_to(struct process *proc);
	
	/*
	 * Function: user_mode
	 * 
	 * Switches execution back to user mode.
	 * 
	 * Parameters:
	 * 
	 *     entry - Entry point to jump to.
	 *     sp    - Stack pointer address.
	 * 
	 * Description:
	 * 
	 *     The <user_mode> function switches execution back to user mode, 
	 *     jumping to the address _entry_ and setting the stack pointer to
	 *     _sp_.
	 * 
	 * See also:
	 * 
	 *     <switch_to>
	 */
	EXTERN void user_mode(addr_t entry, addr_t sp);

/*============================================================================*
 * Section: I/O                                                               *
 *============================================================================*/
	
	/*
	 * Function: inputb
	 * 
	 * Reads a byte from a port.
	 * 
	 * Parameters:
	 * 
	 *     port - Input port number.
	 * 
	 * Description:
	 * 
	 *     The <inputb> function reads a byte from the I/O port _port_.
	 * 
	 * Return:
	 * 
	 *     The byte read is returned.
	 * 
	 * See also:
	 * 
	 *     <iowait>, <outputb>
	 */
	EXTERN byte_t inputb(word_t port);
	
	/*
	 * Function: inputw
	 * 
	 * Reads a word from a port.
	 * 
	 * Parameters:
	 * 
	 *     port - Input port number.
	 * 
	 * Description:
	 * 
	 *     The <inputw> function reads a word from the I/O port _port_.
	 * 
	 * Return:
	 * 
	 *     The word read is returned.
	 * 
	 * See also:
	 * 
	 *     <iowait>, <outputw>
	 */
	EXTERN word_t inputw(word_t port);
	
	/*
	 * Function: iowait
	 * 
	 * Forces an I/O operation to complete.
	 * 
	 * Description:
	 * 
	 *     The <iowait> function forces the CPU to wait for an I/O operation to
	 *     complete by  effectively waisting CPU time, that is, performing some
	 *     useless operation.
	 */
	EXTERN void iowait(void);
	
	/*
	 * Function: outputb
	 * 
	 * Writes a byte to a port.
	 * 
	 * Parameters:
	 * 
	 *     port - Output port number.
	 *     byte - Byte to be written.
	 * 
	 * Description:
	 * 
	 *     The <outputb> function writes the byte _byte_ to the I/O port _port_.
	 * 
	 * See also:
	 * 
	 *     <iowait>, <inputb>
	 */
	EXTERN void outputb(word_t port, byte_t byte);
	
	/*
	 * Function: outputw
	 * 
	 * Writes a word to a port.
	 * 
	 * Parameters:
	 * 
	 *     port - Output port number.
	 *     word - Word to be written.
	 * 
	 * Description:
	 * 
	 *     The <outputw> function writes the word _word_ to the I/O port _port_.
	 * 
	 * See also:
	 * 
	 *     <iowait>, <inputw>
	 */
	EXTERN void outputw(word_t port, word_t word);

/*============================================================================*
 * Section: Memory                                                            *
 *============================================================================*/
 
	/*
	 * Function: physcpy
	 * 
	 * Performs a physical memory copy.
	 * 
	 * Description:
	 * 
	 *     The <physcpy> function copies _n_ bytes from memory area _src_ to
	 *     memory area _dest_. Both addresses are interpreted as being physical 
	 *     addresses, thus a physical memory copy is performed.
	 * 
	 * Notes:
	 * 
	 *     - The memory areas must not overlap.
	 */
	EXTERN void physcpy(addr_t dest, addr_t src, size_t n);
 
#endif /* HAL_H_ */

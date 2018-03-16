/*
 * Copyright(C) 2011-2018 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2018 Davidson Francis <davidsondfgl@gmail.com>
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

/* Must come first. */
#define _ASM_FILE_

#include <or1k/or1k.h>
#include <or1k/asm_defs.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>

/* Exported symbols. */
.globl gdt_flush
.globl idt_flush
.globl tss_flush
.globl tlb_flush
.globl enable_interrupts
.globl disable_interrupts
.globl halt
.globl physcpy
.globl switch_to
.globl user_mode
.globl fpu_init
.globl pmc_init
.globl read_pmc
.globl write_msr
.globl mfspr
.globl mtspr

/* Imported symbols. */
.globl processor_reload

/*----------------------------------------------------------------------------*
 *                                 tlb_flush                                  *
 *----------------------------------------------------------------------------*/

/*
 * Flushes the TLB.
 */
tlb_flush:
	LOAD_SYMBOL_2_GPR(r13, SPR_DTLBMR_BASE(0))
	LOAD_SYMBOL_2_GPR(r15, SPR_ITLBMR_BASE(0))
		
	l.addi	r17, r0, 128
	.loop:
		l.mtspr	r13, r0, 0x0
		l.mtspr	r15, r0, 0x0

		l.addi	r13, r13, 1
		l.addi	r15, r15, 1
		l.sfeq	r17, r0
		l.bnf	.loop
		l.addi	r17, r17, -1

	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                            enable_interrupts()                             *
 *----------------------------------------------------------------------------*/
 
/*
 * Enables all hardware interrupts.
 */
enable_interrupts:
	l.addi r1,  r1, -4
	l.sw 0(r1), r3

	l.mfspr	r3, r0, SPR_SR
	l.ori	r3, r3, lo(SPR_SR_IEE | SPR_SR_TEE)
	l.mtspr	r0, r3, SPR_SR

	l.lwz  r3, 0(r1)
	l.addi r1, r1, 4

	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                            disable_interrupts()                            *
 *----------------------------------------------------------------------------*/
 
/*
 * Disables all hardware interrupts.
 */
disable_interrupts:
	l.addi r1,  r1, -8
	l.sw 4(r1), r3
	l.sw 0(r1), r4

	l.mfspr r4, r0, SPR_SR
	l.movhi	r3, hi(~(SPR_SR_IEE|SPR_SR_TEE))
	l.ori	r3, r3, lo(~(SPR_SR_IEE|SPR_SR_TEE))
	l.and   r4, r4, r3
	l.mtspr r0, r4, SPR_SR

	l.lwz  r3, 4(r1)	
	l.lwz  r4, 0(r1)
	l.addi r1, r1, 8

	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                   halt()                                   *
 *----------------------------------------------------------------------------*/
 
/*
 * Halts the processor.
 */
halt:
	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                 physcpy()                                  *
 *----------------------------------------------------------------------------*/

/*
 * Physical memory copy.
 */
physcpy:
	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                switch_to()                                 *
 *----------------------------------------------------------------------------*/

/*
 * Switches execution to other process.
 */
switch_to:
	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                 user_mode()                                *
 *----------------------------------------------------------------------------*/

/*
 * Switches to user mode.
 */
user_mode:
	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                  pmc_init()                                *
 *----------------------------------------------------------------------------*/

/*
 * Reset the PMC counters to a known state.
 */
pmc_init:
	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                  read_pmc()                                *
 *----------------------------------------------------------------------------*/

/*
 * Reset a specified performance counter.
 */
read_pmc:
	l.ori r11, r0, 0
	l.jr  r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                  read_msr()                                *
 *----------------------------------------------------------------------------*/

/*
 * Reads from an specified MSR register.
 */
read_msr:
	l.ori r11, r0, 0
	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                  write_msr()                               *
 *----------------------------------------------------------------------------*/

/*
 * Writes to an specified MSR register.
 */
write_msr:
	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                    mfspr()                                 *
 *----------------------------------------------------------------------------*/

/*
 * Reads from an specified Special-Purpose register.
 */
mfspr:
	l.mfspr r11, r3, 0
	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                    mtspr()                                 *
 *----------------------------------------------------------------------------*/

/*
 * Writes to an specified Special-Purpose register.
 */
mtspr:
	l.mtspr r3, r4, 0
	l.jr r9
	l.nop

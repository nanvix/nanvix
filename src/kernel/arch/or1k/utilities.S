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
#include <or1k/int.h>
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
	l.sw   -8(r1), r2
	l.sw   -4(r1), r9
	l.addi r2, r1, 0

	/* Disable paging. */
	l.mfspr r13, r0,   SPR_SR
	l.andi  r13, r13, ~ENABLE_MMU
	l.mtspr r0,  r13,  SPR_SR

	/* Jump to lower-half kernel. */
	LOAD_SYMBOL_2_GPR(r15, KBASE_VIRT)
	LOAD_SYMBOL_2_GPR(r17, lower_kernel)
	l.sub r17, r17, r15
	l.jr  r17
	l.nop
	lower_kernel:

physcpy.loop:
	l.lwz r19,  0(r4)
	l.sw 0(r3), r19
	l.addi r4, r4,  4
	l.addi r3, r3,  4
	l.addi r5, r5, -4
	l.sfnei r5, 0
	l.bf physcpy.loop
	l.nop

	/* Re-enable paging. */
	l.mfspr r13, r0,  SPR_SR
	l.ori   r13, r13, ENABLE_MMU
	l.mtspr r0,  r13, SPR_SR

	/* Come back to higher-half kernel. */
	LOAD_SYMBOL_2_GPR(r17, higher_kernel)
	l.jr r17
	l.nop
	higher_kernel:

	l.ori r1, r2, 0
	l.lwz r2, -8(r1)
	l.lwz r9, -4(r1)
	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                switch_to()                                 *
 *----------------------------------------------------------------------------*/

/*
 * Switches execution to other process.
 */
switch_to:
	
	/* Save process context. */
	LOAD_SYMBOL_2_GPR(r13, curr_proc)
	l.lwz r13, 0(r13)
	l.lwz r15, PROC_KESP(r13)

	l.addi r1,  r1, -8
	l.sw 0(r1), r2
	l.sw 4(r1), r15
	l.sw PROC_KESP(r13), r1

	/* Switch processes. */
	LOAD_SYMBOL_2_GPR(r13, curr_proc)
	l.sw 0(r13), r3

	/* Load process context. */
	l.lwz r1, PROC_KESP(r3)

	/* New process?. */
	l.lwz  r15, PROC_FLAGS(r3)
	l.ori  r17, r0,  (1 << PROC_NEW)
	l.and  r19, r15, r17

	l.xori r17, r17, -1
	l.and  r17, r15, r17
	l.sw   PROC_FLAGS(r3), r17

	l.sfnei r19, 0
	l.bf fork_return
	l.nop

	/* Restore stack. */
	l.lwz  r15, 4(r1)
	l.lwz  r2,  0(r1)
	l.addi r1, r1, 8
	l.sw PROC_KESP(r3), r15

	l.jr r9
	l.nop

/*
 * Returns from fork.
 */
fork_return:
	LOAD_SYMBOL_2_GPR(r15, leave)
	l.sw GPR11(r1), r0
	l.jr r15
	l.nop

/*----------------------------------------------------------------------------*
 *                                 user_mode()                                *
 *----------------------------------------------------------------------------*/

/*
 * Switches to user mode.
 */
user_mode:

	/* Set interrupt level to "user level". */
	LOAD_SYMBOL_2_GPR(r13, curr_proc)
	l.lwz r13, 0(r13)
	l.sw PROC_INTLVL(r13), r0	

	/* Load data. */
	LOAD_SYMBOL_2_GPR(r15, USER_SR)
	l.mtspr r0, r15, SPR_ESR_BASE  /* Status register. */
	l.mtspr r0, r3, SPR_EPCR_BASE  /* Entry point.     */
	l.or r1, r0, r4                /* Stack.           */

	l.rfe
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

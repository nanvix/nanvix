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
#include <nanvix/smp.h>

/* Exported symbols. */
.globl gdt_flush
.globl idt_flush
.globl tss_flush
.globl tlb_flush
.globl setup_interrupts
.globl enable_interrupts
.globl disable_interrupts
.globl halt
.globl physcpy
.globl switch_to
.globl user_mode
.globl forge_stack
.globl fpu_init
.globl pmc_init
.globl read_pmc
.globl write_msr
.globl mfspr
.globl mtspr
.globl spin_init
.globl spin_lock
.globl spin_unlock
.globl save_ipi_context

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
 *                            setup_interrupts()                              *
 *----------------------------------------------------------------------------*/
 
/*
 * Enables and unmask all hardware interrupts.
 */
setup_interrupts:
	
	/* Unmask HW ints. */
	l.ori   r13, r0, lo((1 << INT_COM1) | (1 << INT_OMPIC))
	l.mtspr r0, r13, SPR_PICMR

	/* Enable Timer and HW interrupts. */
	l.mfspr	r13, r0, SPR_SR
	l.ori	r13, r13, lo(SPR_SR_IEE|SPR_SR_TEE)
	l.mtspr	r0, r13, SPR_SR

	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                            enable_interrupts()                             *
 *----------------------------------------------------------------------------*/
 
/*
 * Enables all hardware interrupts.
 */
enable_interrupts:

	/* Enable HW interrupts. */
	l.mfspr	r13, r0, SPR_SR
	l.ori	r13, r13, lo(SPR_SR_IEE|SPR_SR_TEE)
	l.mtspr	r0, r13, SPR_SR

	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                            disable_interrupts()                            *
 *----------------------------------------------------------------------------*/
 
/*
 * Disables all hardware interrupts.
 */
disable_interrupts:

	/* Disable HW interrupts. */
	l.mfspr r15, r0, SPR_SR
	l.movhi	r13, hi(~(SPR_SR_IEE|SPR_SR_TEE))
	l.ori	r13, r13, lo(~(SPR_SR_IEE|SPR_SR_TEE))
	l.and   r15, r15, r13
	l.mtspr r0, r15, SPR_SR

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
	l.addi r2, r1,  0
	l.addi r1, r1, -8

	/* Critical section. */
	l.jal disable_interrupts
	l.nop

	/* Jump to lower-half kernel. */
	LOAD_SYMBOL_2_GPR(r13, KBASE_VIRT)
	LOAD_SYMBOL_2_GPR(r15, lower_kernel)
	l.sub r15, r15, r13
	l.jr  r15
	l.nop
	lower_kernel:

	/* Disable paging. */
	l.movhi	r13, hi(~(ENABLE_MMU))
	l.ori	r13, r13, lo(~(ENABLE_MMU))

	l.mfspr r15, r0,  SPR_SR
	l.and   r15, r15, r13
	l.mtspr r0,  r15, SPR_SR

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

	/* Critical section. */
	l.jal enable_interrupts
	l.nop

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
	/* We need to save SPR_SR before disable_interrupts. */
	l.mfspr r29, r0, SPR_SR

	/* Critical region. */
	l.or r31, r9, r0
	l.jal disable_interrupts
	l.nop
	l.or r9, r31, r0

	/* Save process context. */
	LOAD_SYMBOL_2_GPR(r13, cpus)
	l.mfspr r15, r0,  SPR_COREID
	l.slli  r15, r15, PERCORE_SIZE_LOG2
	l.add   r15, r13, r15

	l.lwz   r13, PERCORE_CURRTHREAD(r15)
	l.sfeqi r13, 0
	l.bf 1f
	l.nop
	l.lwz r15, THRD_KESP(r13)

	/* Preserve registers state. */
	l.addi r1, r1, -60
	l.sw  0(r1), r29
	l.sw  4(r1), r2
	l.sw  8(r1), r15
	l.sw 12(r1), r9
	l.sw 16(r1), r10
	l.sw 20(r1), r12
	l.sw 24(r1), r14
	l.sw 28(r1), r16
	l.sw 32(r1), r18
	l.sw 36(r1), r20
	l.sw 40(r1), r22
	l.sw 44(r1), r24
	l.sw 48(r1), r26
	l.sw 52(r1), r28
	l.sw 56(r1), r30
	l.sw THRD_KESP(r13), r1
1:
	/* Switch processes. */
	LOAD_SYMBOL_2_GPR(r13, curr_proc)
	l.sw 0(r13), r3

	/* Switch thread */
	LOAD_SYMBOL_2_GPR(r14, cpus)
	l.mfspr r15, r0,  SPR_COREID
	l.slli  r15, r15, PERCORE_SIZE_LOG2
	l.add   r15, r14, r15
	l.addi  r14, r15, PERCORE_CURRTHREAD
	l.sw 0(r14), r4

	/**
	 * Load process context
	 *
	 * There are 2 possible contexts to be restored:
	 * 1) Core master returning from an old
	 *    IPI context.
	 *
	 * 2) Core slave (or master in UP)
	 *    returning a previous execution.
	 *
	 * In the first, the IPI kstack is restored
	 * and in the later, the normal kernel thread.
	 */
	LOAD_SYMBOL_2_GPR(r14, smp_enabled)
	l.lwz   r14, 0(r14)
	l.sfeqi r14, 0
	l.bf 2f
	l.nop

	l.mfspr r14, r0, SPR_COREID
	l.sfnei r14, CORE_MASTER
	l.bf 2f
	l.nop

	l.lwz   r14, THRD_FLAGS(r4)
	l.andi  r14, r14, (1 << THRD_SYS)
	l.sfeqi r14, 0
	l.bf 2f
	l.nop

	l.lwz r1, THRD_IPISTACK(r4)
	LOAD_SYMBOL_2_GPR(r14, leave.out)
	l.jr r14
	l.nop
2:
	l.lwz r1, THRD_KESP(r4)
3:
	l.jal tlb_flush
	l.nop

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

	/* New thread?. */
	l.lwz  r15, THRD_FLAGS(r4)
	l.ori  r17, r0,  (1 << THRD_NEW)
	l.and  r19, r15, r17

	l.xori r17, r17, -1
	l.and  r17, r15, r17
	l.sw   THRD_FLAGS(r4), r17

	l.sfnei r19, 0
	l.bf pthread_create_return
	l.nop

	/* Restore stack. */
	l.lwz r30, 56(r1)
	l.lwz r28, 52(r1)
	l.lwz r26, 48(r1)
	l.lwz r24, 44(r1)
	l.lwz r22, 40(r1)
	l.lwz r20, 36(r1)
	l.lwz r18, 32(r1)
	l.lwz r16, 28(r1)
	l.lwz r14, 24(r1)
	l.lwz r12, 20(r1)
	l.lwz r10, 16(r1)
	l.lwz r9,  12(r1)
	l.lwz r15,  8(r1)
	l.lwz r2,   4(r1)
	l.lwz r29, 0(r1)
	l.ori r29, r29, ENABLE_MMU
	l.mtspr r0, r29, SPR_SR

	l.addi r1, r1, 60

	l.sw THRD_KESP(r4), r15

	/* Processor reload. */
	l.addi r1, r1, -4
	l.sw   0(r1), r9
	
	LOAD_SYMBOL_2_GPR(r13, processor_reload)
	l.jalr r13
	l.nop
	
	l.lwz  r9, 0(r1)
	l.addi r1, r1, 4

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

/*
 * Returns from pthread_create.
 */
pthread_create_return:
	/* Decrement interrupt level. */
	LOAD_SYMBOL_2_GPR(r3, cpus)
	l.mfspr r4, r0, SPR_COREID
	l.slli  r4, r4, PERCORE_SIZE_LOG2
	l.add   r4, r3, r4
	l.lwz   r3, PERCORE_CURRTHREAD(r4)

    l.lwz   r4, THRD_INTLVL(r3)
    l.addi  r4, r4, -1
    l.sw THRD_INTLVL(r3), r4

	LOAD_SYMBOL_2_GPR(r15, leave.out)
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
	l.sw   -8(r1), r2
	l.sw   -4(r1), r9
	l.addi r2, r1,  0
	l.addi r1, r1, -8

	/* Enter critical region. */
	l.jal disable_interrupts
	l.nop

	/* Set interrupt level to "user level". */
	LOAD_SYMBOL_2_GPR(r13, cpus)
	LOAD_SYMBOL_2_GPR(r14, curr_core)
	l.lwz  r14, 0(r14)
	l.slli r14, r14, PERCORE_SIZE_LOG2
	l.add  r14, r13, r14
	l.lwz  r13, PERCORE_CURRTHREAD(r14)
	l.ori  r14, r0, 1
	l.sw   THRD_INTLVL(r13), r14

	/* Reset flags. */
	l.sw THRD_FLAGS(r13), r0

	/* Retrieves the intstack. */
	l.lwz r13, THRD_INTSTACK(r13)

	/* Load data. */
	LOAD_SYMBOL_2_GPR(r15, USER_SR)
	l.sw ESR(r13), r15   /* Status register. */
	l.sw EPCR(r13), r3   /* Entry point.     */
	l.sw SP(r13),   r4   /* Stack.           */
	l.sw GPR2(r13), r4   /* Frame pointer.   */

	/* Configure argc, argv and envp. */	
	l.or r15, r4, r0
	l.lwz r3,  4(r15)  /* argc. */
	l.lwz r4,  8(r15)  /* argv. */
	l.lwz r5, 12(r15)  /* argp. */
	l.sw GPR3(r13), r3
	l.sw GPR4(r13), r4
	l.sw GPR5(r13), r5

	/* Clear registers. */
	l.sw R0(r13),    r0
	l.sw GPR6(r13) , r0
	l.sw GPR7(r13) , r0
	l.sw GPR8(r13) , r0
	l.sw GPR9(r13) , r0
	l.sw GPR10(r13), r0
	l.sw GPR11(r13), r0
	l.sw GPR12(r13), r0
	l.sw GPR13(r13), r0
	l.sw GPR14(r13), r0
	l.sw GPR15(r13), r0
	l.sw GPR16(r13), r0
	l.sw GPR17(r13), r0
	l.sw GPR18(r13), r0
	l.sw GPR19(r13), r0
	l.sw GPR20(r13), r0
	l.sw GPR21(r13), r0
	l.sw GPR22(r13), r0
	l.sw GPR23(r13), r0
	l.sw GPR24(r13), r0
	l.sw GPR25(r13), r0
	l.sw GPR26(r13), r0
	l.sw GPR27(r13), r0
	l.sw GPR28(r13), r0
	l.sw GPR29(r13), r0
	l.sw GPR30(r13), r0
	l.sw GPR31(r13), r0
	l.sw EEAR(r13),  r0

	l.ori r1, r2, 0
	l.lwz r2, -8(r1)
	l.lwz r9, -4(r1)
	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                  forge_stack()                             *
 *----------------------------------------------------------------------------*/

/*
 * Forge the expected stack for a new thread.
 */
forge_stack:

	/* Adjust kern_sp using kstack page */
	l.addi r3, r3,  PAGE_SIZE
	l.addi r3, r3, -DWORD_SIZE
	l.addi r3, r3, -INT_FRAME_SIZE
	l.or r11, r3, r0 /* kern_sp pointer will be returned */

	/* General Purpose registers. */
	l.sw R0(r3)   , r0
	l.sw SP(r3)   , r5
	l.sw GPR2(r3) , r5 /* First frame so SP and FP are the same.  */
	l.sw GPR3(r3) , r4 /* start_thread 1st param : start_routine. */
	l.sw GPR4(r3) , r6 /* start_thread 2nd param : argument.      */
	l.sw GPR5(r3) , r0
	l.sw GPR6(r3) , r0
	l.sw GPR7(r3) , r0
	l.sw GPR8(r3) , r0
	l.sw GPR9(r3) , r0
	l.sw GPR10(r3), r0
	l.sw GPR11(r3), r0
	l.sw GPR12(r3), r0
	l.sw GPR13(r3), r0
	l.sw GPR14(r3), r0
	l.sw GPR15(r3), r0
	l.sw GPR16(r3), r0
	l.sw GPR17(r3), r0
	l.sw GPR18(r3), r0
	l.sw GPR19(r3), r0
	l.sw GPR20(r3), r0
	l.sw GPR21(r3), r0
	l.sw GPR22(r3), r0
	l.sw GPR23(r3), r0
	l.sw GPR24(r3), r0
	l.sw GPR25(r3), r0
	l.sw GPR26(r3), r0
	l.sw GPR27(r3), r0
	l.sw GPR28(r3), r0
	l.sw GPR29(r3), r0
	l.sw GPR30(r3), r0
	l.sw GPR31(r3), r0

	/* EPCR. */
	l.sw EPCR(r3), r7 /* start_thread initialization routine. */

	/* EEAR. */
	l.sw EEAR(r3), r0 /* Not used. */

	/* ESR. */
	LOAD_SYMBOL_2_GPR(r15, USER_SR)
	l.sw ESR(r3), r15

	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                              save_ipi_context()                            *
 *----------------------------------------------------------------------------*/

/*
 * Saves the current context for the master core into the
 * current thread IPI stack.
 */
save_ipi_context:
	l.sw   -8(r1), r2
	l.sw   -4(r1), r9
	l.addi r2, r1,  0
	l.addi r1, r1, -16

	/* Scratch registers. */
	l.sw -12(r2), r3
	l.sw -16(r2), r4

	/* Retrieve the IPI stack. */
	LOAD_SYMBOL_2_GPR(r3, cpus)
	LOAD_SYMBOL_2_GPR(r4, curr_core)
	l.lwz  r4, 0(r4)
	l.slli r4, r4, PERCORE_SIZE_LOG2
	l.add  r4, r3, r4
	l.lwz  r3, PERCORE_CURRTHREAD(r4)
	l.lwz  r3, THRD_IPISTACK(r3)

	/* General purpose registers. */
	l.sw R0(r3)   , r0
	l.sw SP(r3)   , r1
	l.sw GPR2(r3) , r2
	l.sw GPR5(r3) , r5
	l.sw GPR6(r3) , r6
	l.sw GPR7(r3) , r7
	l.sw GPR8(r3) , r8
	l.sw GPR9(r3) , r9
	l.sw GPR10(r3), r10
	l.sw GPR11(r3), r11
	l.sw GPR12(r3), r12
	l.sw GPR13(r3), r13
	l.sw GPR14(r3), r14
	l.sw GPR15(r3), r15
	l.sw GPR16(r3), r16
	l.sw GPR17(r3), r17
	l.sw GPR18(r3), r18
	l.sw GPR19(r3), r19
	l.sw GPR20(r3), r20
	l.sw GPR21(r3), r21
	l.sw GPR22(r3), r22
	l.sw GPR23(r3), r23
	l.sw GPR24(r3), r24
	l.sw GPR25(r3), r25
	l.sw GPR26(r3), r26
	l.sw GPR27(r3), r27
	l.sw GPR28(r3), r28
	l.sw GPR29(r3), r29
	l.sw GPR30(r3), r30
	l.sw GPR31(r3), r31

	/* EPCR. */
	LOAD_SYMBOL_2_GPR(r4, ret_from_ipi_context)
	l.sw EPCR(r3), r4

	/* EEAR. */
	l.mfspr r4, r0, SPR_EEAR_BASE
	l.sw EEAR(r3), r4

	/* ESR. */
	l.mfspr r4, r0, SPR_ESR_BASE
	l.sw ESR(r3), r4

	/* Save scratch registers. */
	l.or  r5, r0, r3
	l.lwz r3, -12(r2)
	l.lwz r4, -16(r2)
	l.sw GPR3(r5), r3
	l.sw GPR4(r5), r4

	/* Reset curr_core. */
	LOAD_SYMBOL_2_GPR(r3, curr_core)
	LOAD_SYMBOL_2_GPR(r4, CORE_MASTER)
	l.sw 0(r3), r4

	l.j idle_smp
	l.nop

ret_from_ipi_context:
	
	l.ori r1, r2, 0
	l.lwz r2, -8(r1)
	l.lwz r9, -4(r1)
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

/*----------------------------------------------------------------------------*
 *                                   spin_init                                *
 *----------------------------------------------------------------------------*/

/*
 * Initializes the spin-lock with a given lock.
 */
spin_init:
	l.sw 0(r3), r0
	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                   spin_lock                                *
 *----------------------------------------------------------------------------*/

/*
 * Spin-locks until the lock becomes 0, and so, locks again.
 * lock = 0, unlocked
 * lock = 1, locked
 */
spin_lock:
	l.ori   r13, r0, 1
1:
	l.lwa   r15, 0(r3)
	l.sfeqi r15, 0
	l.bnf   1b
	l.nop
	l.swa 0(r3), r13
	l.bnf   1b
	l.nop

	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                  spin_unlock                               *
 *----------------------------------------------------------------------------*/

/*
 * Unlocks the spin-lock.
 */
spin_unlock:
	l.lwa r13, 0(r3)
	l.swa 0(r3), r0
	l.bnf spin_unlock
	l.nop

	l.jr r9
	l.nop

/*
 * Copyright(C) 2011-2018 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * Copyright(C) 2017-2018 Davidson Francis <davidsondfgl@gmail.com>
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

/*
 * OpenRISC head.S
 *
 * Linux architectural port borrowing liberally from similar works of
 * others.  All original copyrights apply as per the original source
 * declaration.
 *
 * Modifications for the OpenRISC architecture:
 * Copyright (C) 2003 Matjaz Breskvar <phoenix@bsemi.com>
 * Copyright (C) 2010-2011 Jonas Bonn <jonas@southpole.se>
 *
 *      This program is free software; you can redistribute it and/or
 *      modify it under the terms of the GNU General Public License
 *      as published by the Free Software Foundation; either version
 *      2 of the License, or (at your option) any later version.
 */

/* Must come first. */
#define _ASM_FILE_

#include <nanvix/mm.h>
#include <nanvix/syscall.h>
#include <or1k/or1k.h>
#include <or1k/asm_defs.h>
#include <or1k/int.h>
#include <errno.h>

/* Exported symbols. */
.globl swint0
.globl swint1
.globl swint2
.globl swint3
.globl swint4
.globl swint5
.globl swint6
.globl swint7
.globl swint8
.globl swint9
.globl swint10
.globl swint11
.globl swint12
.globl swint13
.globl swint14
.globl swint15
.globl swint16
.globl swint17
.globl syscall
.globl hwint0
.globl hwint1
.globl hwint2
.globl hwint3
.globl hwint4
.globl hwint5
.globl hwint6
.globl hwint7
.globl hwint8
.globl hwint9
.globl hwint10
.globl hwint11
.globl hwint12
.globl hwint13
.globl hwint14
.globl hwint15
.globl leave
.globl leave.out
.globl do_hwint

.globl dtlb_miss_handler
.globl itlb_miss_handler

.section .exception

/*----------------------------------------------------------------------------*
 *                                  save()                                    *
 *----------------------------------------------------------------------------*/

/*
 * Saves registers.
 */
.macro save
	/* General Purpose registers, except r30. */
	l.sw GPR2(r1) , r2
	l.sw GPR3(r1) , r3
	l.sw GPR4(r1) , r4
	l.sw GPR5(r1) , r5
	l.sw GPR6(r1) , r6
	l.sw GPR7(r1) , r7
	l.sw GPR8(r1) , r8
	l.sw GPR9(r1) , r9
	l.sw GPR10(r1), r10
	l.sw GPR11(r1), r11
	l.sw GPR12(r1), r12
	l.sw GPR13(r1), r13
	l.sw GPR14(r1), r14
	l.sw GPR15(r1), r15
	l.sw GPR16(r1), r16
	l.sw GPR17(r1), r17
	l.sw GPR18(r1), r18
	l.sw GPR19(r1), r19
	l.sw GPR20(r1), r20
	l.sw GPR21(r1), r21
	l.sw GPR22(r1), r22
	l.sw GPR23(r1), r23
	l.sw GPR24(r1), r24
	l.sw GPR25(r1), r25
	l.sw GPR26(r1), r26
	l.sw GPR27(r1), r27
	l.sw GPR28(r1), r28
	l.sw GPR29(r1), r29
	l.sw GPR31(r1), r31

	/* Stack Pointer, r1. */
	EXCEPTION_T_LOAD_SP(r30)
	l.sw SP(r1), r30

	/* EPCR. */
	l.mfspr r30, r0, SPR_EPCR_BASE
	l.sw EPCR(r1), r30

	/* EEAR. */
	l.mfspr r30, r0, SPR_EEAR_BASE
	l.sw EEAR(r1), r30

	/* ESR. */
	l.mfspr r30, r0, SPR_ESR_BASE
	l.sw ESR(r1), r30

	/* Restore and save r30. */
	EXCEPTION_T_LOAD_GPR30(r30)
	l.sw GPR30(r1), r30

.endm

/*----------------------------------------------------------------------------*
 *                                 enter()                                    *
 *----------------------------------------------------------------------------*/

/*
 * Enters in kernel.
 */
.macro enter
	LOAD_SYMBOL_2_GPR(r3, curr_proc)
	LOAD_SYMBOL_2_GPR(r5, KBASE_VIRT)

	/* Increment interrupt level. */
	l.sub r3, r3, r5
	l.lwz r3, 0(r3)
	l.sub r3, r3, r5 

	l.lwz  r7, PROC_INTLVL(r3)
	l.addi r7, r7, 1
	l.sw   PROC_INTLVL(r3), r7
    
	/* Save interrupt stack. */
	LOAD_SYMBOL_2_GPR(r3, curr_thread)
	l.sub r3, r3, r5
	l.lwz r3, 0(r3)
	l.sub r3, r3, r5
	l.lwz r7,   THRD_KESP(r3)
	l.sw 0(r1), r7

	l.or  r6, r0, r1
	l.add r6, r5, r6
	l.sw THRD_KESP(r3), r6

.endm

/*----------------------------------------------------------------------------*
 *                                exception_enter                             *
 *----------------------------------------------------------------------------*/

/*
 * Restores the appropriate stack, save registers
 * and enable MMU.
 */
.macro exception_enter
	EXCEPTION_STORE_GPR3
	EXCEPTION_STORE_GPR5

	/* 
	 * Check if we are running in the slave core, i.e:
	 * cores other than 0.
	 */
	l.mfspr r3, r0, SPR_COREID
	l.sfeqi r3, 0
	l.bf 2f
	l.nop

	/* Core != 0, we should send an IPI to core 0. */
1:
	l.j 1b
	l.nop

2:		
	/* Checks if we are in user or kernel mode. */
	LOAD_SYMBOL_2_GPR(r5, KBASE_VIRT)
	EXCEPTION_T_STORE_GPR30           /* Backup register.          */
	l.mfspr r30, r0,  SPR_ESR_BASE
	l.andi  r30, r30, SPR_SR_SM
	l.sfeqi r30, 0
	l.bnf   3f
	EXCEPTION_T_STORE_SP              /* Delay slot, save SP (r1). */
	
	/* Extracts the Kernel stack. */

	/* User mode. */
	LOAD_SYMBOL_2_GPR(r3, curr_thread)
	l.sub r3, r3, r5
	l.lwz r3, 0(r3)
	l.sub r3, r3, r5
	l.lwz r1, THRD_KESP(r3)

	/* Kernel mode. */
3:
	/* Create new stack frame. */
	l.addi r1, r1, -(INT_FRAME_SIZE)
	l.sub  r1, r1, r5

	/* Save registers. */
	EXCEPTION_LOAD_GPR3
	EXCEPTION_LOAD_GPR5
	save

	/* Enter. */
	enter

	/* Enable MMU. */
	l.mfspr r30, r0,  SPR_SR
	l.ori   r30, r30, (ENABLE_MMU | SPR_SR_SM)
	l.mtspr r0,  r30, SPR_SR

	/* Virtual address. */
	l.add r1, r1, r5
.endm

/*----------------------------------------------------------------------------*
 *                                   swint()                                  *
 *----------------------------------------------------------------------------*/

/* Software interrupts */
.macro swint_handler, number, handler
	swint\number:
		exception_enter

		LOAD_SYMBOL_2_GPR(r5, enable_interrupts)
		l.jalr r5
		l.nop

		/* Check if syscall. */
		l.ori   r3, r0, \number
		l.sfeqi r3, 9
		l.bnf 4f
		l.nop

		/* Syscall. */
		l.j syscall
		l.nop

	4:
		/* Not syscall, lets do swint. */
		l.or   r4, r1, r0
		LOAD_SYMBOL_2_GPR(r5, \handler)
		l.jalr r5
		l.nop

		/* Enter critical region. */
		LOAD_SYMBOL_2_GPR(r5, disable_interrupts)
		l.jalr r5
		l.nop

		/* Return from exception. */
	5:
		l.j leave
		l.nop
.endm

/*----------------------------------------------------------------------------*
 *                                   hwint()                                  *
 *----------------------------------------------------------------------------*/

/* Hardware interrupts */
.macro hwint_handler, number
	hwint\number:
		exception_enter

		/* hwint. */
		l.ori  r3, r0, \number
		LOAD_SYMBOL_2_GPR(r5, do_hwint)
		l.jalr r5
		l.nop

		/* Return from exception. */
		l.j leave
		l.nop
.endm

/*----------------------------------------------------------------------------*
 *                                  exceptions                                *
 *----------------------------------------------------------------------------*/

.org 0x100
	l.j reset_exception
	l.nop

.org 0x200
	l.j bus_exception
	l.nop

.org 0x300
	l.j data_page_fault_exception
	l.nop

.org 0x400
	l.j instruction_page_fault
	l.nop

.org 0x500
	l.j tick_timer_exception
	l.nop

.org 0x600
	l.j alignment_exception
	l.nop

.org 0x700
	l.j illegal_instruction_exception
	l.nop

.org 0x800
	l.j external_interrupt_exception
	l.nop

.org 0x900
	l.j	boot_dtlb_miss_handler
	l.nop

.org 0xa00
	l.j	boot_itlb_miss_handler
	l.nop

.org 0xb00
	l.j range_exception
	l.nop

.org 0xc00
	l.j syscall_exception
	l.nop

.org 0xd00
	l.j floating_point_exception
	l.nop

.org 0xe00
	l.j trap_exception
	l.nop

.align PAGE_SIZE
/*----------------------------------------------------------------------------*
 *                              reset_exception()                             *
 *----------------------------------------------------------------------------*/
reset_exception:
	l.or r1, r0, r0
	l.or r2, r0, r0
	l.or r3, r0, r0
	l.or r4, r0, r0
	l.or r5, r0, r0
	CLEAR_ALL_GPR
	l.j start
	l.nop

/*----------------------------------------------------------------------------*
 *                                bus_exception()                             *
 *----------------------------------------------------------------------------*/
bus_exception:
	swint_handler 1 do_bus_error

/*----------------------------------------------------------------------------*
 *                         data_page_fault_exception()                        *
 *----------------------------------------------------------------------------*/
data_page_fault_exception:
	swint_handler 2 _do_page_pfault

/*----------------------------------------------------------------------------*
 *                           instruction_page_fault()                         *
 *----------------------------------------------------------------------------*/
instruction_page_fault:
	swint_handler 3 _do_page_pfault

/*----------------------------------------------------------------------------*
 *                            tick_timer_exception()                          *
 *----------------------------------------------------------------------------*/
tick_timer_exception:
	hwint_handler INT_CLOCK

/*----------------------------------------------------------------------------*
 *                             alignment_exception()                          *
 *----------------------------------------------------------------------------*/
alignment_exception:
	swint_handler 5 do_alignment

/*----------------------------------------------------------------------------*
 *                       illegal_instruction_exception()                      *
 *----------------------------------------------------------------------------*/
illegal_instruction_exception:
	swint_handler 6 do_illegal_instruction

/*----------------------------------------------------------------------------*
 *                        external_interrupt_exception()                      *
 *----------------------------------------------------------------------------*/
external_interrupt_exception:
	hwint_handler INT_EXTERNAL

/*----------------------------------------------------------------------------*
 *                               range_exception()                            *
 *----------------------------------------------------------------------------*/
range_exception:
	swint_handler 8 do_range

/*----------------------------------------------------------------------------*
 *                              syscall_exception()                           *
 *----------------------------------------------------------------------------*/
syscall_exception:
	swint_handler 9 0

/*----------------------------------------------------------------------------*
 *                           floating_point_exception()                       *
 *----------------------------------------------------------------------------*/
floating_point_exception:
	swint_handler 10 do_float

/*----------------------------------------------------------------------------*
 *                                trap_exception()                            *
 *----------------------------------------------------------------------------*/
trap_exception:
	swint_handler 11 do_trap

/*----------------------------------------------------------------------------*
 *                                leave()                                     *
 *----------------------------------------------------------------------------*/

/*
 * Leaves kernel.
 */
leave:
	LOAD_SYMBOL_2_GPR(r3, curr_thread)
	l.lwz r3, 0(r3)

	/* Restore interrupt stack. */
	l.lwz  r4, 0(r1)
	l.sw THRD_KESP(r3), r4

	LOAD_SYMBOL_2_GPR(r3, curr_proc)
	l.lwz r3, 0(r3)

	/* Decrement interrupt level. */
	l.lwz   r4, PROC_INTLVL(r3)
	l.addi  r4, r4, -1
	l.sw PROC_INTLVL(r3), r4
	l.sfnei r4, 0
	l.bf leave.out
	l.nop

	/* Check if we were handling a system call. */
	l.lwz  r15, PROC_FLAGS(r3)
	l.ori  r17, r0,  (1 << PROC_SYS)
	l.and  r19, r15, r17
	l.xori r17, r17, -1
	l.and  r17, r15, r17
	l.sw   PROC_FLAGS(r3), r17
	l.sfnei r19, 0
	l.bnf check_signals
	l.nop

	/*
	 * The kernel is non-preemptive.
	 * So, let us be nice with other processes
	 * and give them a chance to run.
	 */
	LOAD_SYMBOL_2_GPR(r15, yield)
	l.jalr r15
	l.nop

	/* TODO: Check signals. */
	check_signals:
		l.nop
		l.nop

leave.out:
	/* General Purpose registers, except r30 and r31. */
	l.lwz r2 , GPR2(r1)
	l.lwz r3 , GPR3(r1)
	l.lwz r4 , GPR4(r1)
	l.lwz r5 , GPR5(r1)
	l.lwz r6 , GPR6(r1)
	l.lwz r7 , GPR7(r1)
	l.lwz r8 , GPR8(r1)
	l.lwz r9 , GPR9(r1)
	l.lwz r10, GPR10(r1)
	l.lwz r11, GPR11(r1)
	l.lwz r12, GPR12(r1)
	l.lwz r13, GPR13(r1)
	l.lwz r14, GPR14(r1)
	l.lwz r15, GPR15(r1)
	l.lwz r16, GPR16(r1)
	l.lwz r17, GPR17(r1)
	l.lwz r18, GPR18(r1)
	l.lwz r19, GPR19(r1)
	l.lwz r20, GPR20(r1)
	l.lwz r21, GPR21(r1)
	l.lwz r22, GPR22(r1)
	l.lwz r23, GPR23(r1)
	l.lwz r24, GPR24(r1)
	l.lwz r25, GPR25(r1)
	l.lwz r26, GPR26(r1)
	l.lwz r27, GPR27(r1)
	l.lwz r28, GPR28(r1)
	l.lwz r29, GPR29(r1)

	/* SPR_EPCR. */
	l.lwz   r30, EPCR(r1)
	l.mtspr r0,  r30, SPR_EPCR_BASE

	/* SPR_EEAR. */
	l.lwz   r30, EEAR(r1)
	l.mtspr r0,  r30, SPR_EEAR_BASE

	/* SPR_ESR. */
	l.lwz   r30, ESR(r1)
	l.ori   r30, r30, (ENABLE_MMU)
	l.mtspr r0,  r30, SPR_ESR_BASE

	/* Let us check if we have to enable TTMR_IE. */
	l.andi  r30, r30, (SPR_SR_IEE|SPR_SR_TEE)
	l.sfnei r30, (SPR_SR_IEE|SPR_SR_TEE)
	l.bf 1f
	l.nop

	l.mfspr r30, r0, SPR_TTMR
	LOAD_SYMBOL_2_GPR(r31, SPR_TTMR_IE)
	l.or    r30, r30, r31
	l.mtspr r0, r30, SPR_TTMR
1:
	/* r30. */
	l.lwz r30, GPR30(r1)

	/* r31. */
	l.lwz r31, GPR31(r1)

	/* Stack Pointer, r1. */
	l.lwz r1, SP(r1)

	l.rfe

/*----------------------------------------------------------------------------*
 *                                 syscall()                                  *
 *----------------------------------------------------------------------------*/

/*
 * System call hook.
 */
syscall:
	/* Set 'handling system call' flag. */
	LOAD_SYMBOL_2_GPR(r3, curr_proc)
	l.lwz r3, 0(r3)
	l.lwz r5, PROC_FLAGS(r3)
	l.ori r5, r5, (1 << PROC_SYS)
	l.sw  PROC_FLAGS(r3), r5

	/* Get system call parameters. */
	l.lwz r3,  GPR3(r1)
	l.lwz r4,  GPR4(r1)
	l.lwz r5,  GPR5(r1)
	l.lwz r6,  GPR6(r1)
	l.lwz r7,  GPR7(r1)
	l.lwz r11, GPR11(r1) /* syscall number. */
	
	/* Check for bad system call. */
	l.sfltui r11, NR_SYSCALLS
	l.bf good_syscall
	l.nop

	LOAD_SYMBOL_2_GPR(r11, ENOSYS)
	l.sub r11, r0, r11
	l.j bad_syscall
	l.nop

	good_syscall:
		/* Get system call. */
		LOAD_SYMBOL_2_GPR(r13, syscalls_table)
		l.slli r11, r11, 2
		l.add  r11, r11, r13
		l.lwz  r11, 0(r11)

		/* Do system call. */
		l.jalr r11
		l.nop

	bad_syscall:
		/* Copy return value to user stack. */
		l.sw GPR11(r1), r11

		/* Enter critical region. */
		LOAD_SYMBOL_2_GPR(r5, disable_interrupts)
		l.jalr r5
		l.nop

		l.j leave
		l.nop

/*
 * Page protection fault handler wrapper.
 */
_do_page_pfault:
	l.sw   -8(r1), r2
	l.sw   -4(r1), r9
	l.addi r2, r1,  0
	l.addi r1, r1, -8

	l.lwz r3, EEAR(r4) /* addr.     */
	l.or  r7, r4, r0   /* intstack. */
	l.ori r4, r0, 2    /* err.      */
	l.ori r5, r0, 0    /* dummy0.   */
	l.ori r6, r0, 0    /* dummy1.   */

	LOAD_SYMBOL_2_GPR(r13, do_page_fault)
	l.jalr r13
	l.nop

	l.ori r1, r2, 0
	l.lwz r2, -8(r1)
	l.lwz r9, -4(r1)
	l.jr r9
	l.nop

/*
 * Page valid fault handler wrapper.
 */
_do_page_vfault:
	l.sw   -8(r1), r2
	l.sw   -4(r1), r9
	l.addi r2, r1,  0
	l.addi r1, r1, -8

	l.lwz r3, EEAR(r4) /* addr.     */
	l.or  r7, r4, r0   /* intstack. */
	l.ori r4, r0, 0    /* err.      */
	l.ori r5, r0, 0    /* dummy0.   */
	l.ori r6, r0, 0    /* dummy1.   */

	LOAD_SYMBOL_2_GPR(r13, do_page_fault)
	l.jalr r13
	l.nop

	l.ori r1, r2, 0
	l.lwz r2, -8(r1)
	l.lwz r9, -4(r1)
	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                          boot_dtlb_miss_handler                            *
 *----------------------------------------------------------------------------*/
boot_dtlb_miss_handler:

	/*
	 * mask for DTLB_MR register: - (0) sets V (valid) bit,
	 *                            - (31-12) sets bits belonging to VPN (31-12)
	 */
	#define DTLB_MR_MASK 0xfffff001

	/* 
	 * mask for DTLB_TR register: - (2) sets CI (cache inhibit) bit,
	 *                            - (4) sets A (access) bit,
	 *                            - (5) sets D (dirty) bit,
	 *                            - (8) sets SRE (superuser read) bit
	 *                            - (9) sets SWE (superuser write) bit
	 *                            - (31-12) sets bits belonging to VPN (31-12)
	 */
	#define DTLB_TR_MASK 0xfffff332

	/* These are for masking out the VPN/PPN value from the MR/TR registers...
	 * it's not the same as the PFN */
	#define VPN_MASK 0xfffff000
	#define PPN_MASK 0xfffff000

	EXCEPTION_STORE_GPR6
	EXCEPTION_STORE_GPR2
	EXCEPTION_STORE_GPR3
	EXCEPTION_STORE_GPR4
	EXCEPTION_STORE_GPR5

	l.mfspr r4, r0, SPR_EEAR_BASE

	immediate_translation:
		CLEAR_GPR(r6)

		l.srli	r3, r4, 13  /* r3 <- r4 / 8192 (sets are relative to page size (8Kb). */

		l.mfspr r6, r0, SPR_DMMUCFGR
		l.andi	r6, r6, SPR_DMMUCFGR_NTS
		l.srli	r6, r6, SPR_DMMUCFGR_NTS_OFF
		l.ori	r5, r0, 0x1
		l.sll	r5, r5, r6 	/* r5 = number DMMU sets. */
		l.addi	r6, r5, -1  /* r6 = nsets mask.       */
		l.and	r2, r3, r6	/* r2 <- r3 % NSETS_MASK. */

		l.or    r6, r6, r4
		l.ori   r6, r6, ~(VPN_MASK)         /* r6 <- VPN :VPN .xfff - clear up lo(r6) to 0x**** *fff. */
		l.movhi r5, hi(DTLB_MR_MASK)        /* r5 <- ffff:0000.x000.                                  */
		l.ori   r5, r5, lo(DTLB_MR_MASK)    /* r5 <- ffff:1111.x001 - apply DTLB_MR_MASK.             */
		l.and   r5, r5, r6                  /* r5 <- VPN :VPN .x001 - we have DTLBMR entry.           */
		l.mtspr r2, r5, SPR_DTLBMR_BASE(0)  /* set DTLBMR.                                            */

		/* Set up DTLB with no translation for EA <= 0xbfffffff */
		LOAD_SYMBOL_2_GPR(r6, 0xbfffffff)
		l.sfgeu  r6, r4                     /* flag if r6 >= r4 (if 0xbfffffff >= EA).                */
		l.bf     1f                         /* goto out.                                              */
		l.and    r3, r4, r4                 /* delay slot :: 24 <- r4 (if flag==1).                   */

		l.add    r3, r0, r4                 /* r3 <- PA.                                              */
	1:
		l.ori   r3, r3, ~(PPN_MASK)         /* r3 <- PPN :PPN .xfff - clear up lo(r6) to 0x**** *fff. */
		l.movhi r5, hi(DTLB_TR_MASK)        /* r5 <- ffff:0000.x000.                                  */
		l.ori   r5, r5, lo(DTLB_TR_MASK)    /* r5 <- ffff:1111.x330 - apply DTLB_MR_MASK.             */
		l.and   r5, r5, r3                  /* r5 <- PPN :PPN .x330 - we have DTLBTR entry.           */
		l.mtspr r2, r5, SPR_DTLBTR_BASE(0)  /* set DTLBTR.                                            */

		EXCEPTION_LOAD_GPR6
		EXCEPTION_LOAD_GPR5
		EXCEPTION_LOAD_GPR4
		EXCEPTION_LOAD_GPR3
		EXCEPTION_LOAD_GPR2

		l.rfe                               /* SR <- ESR, PC <- EPC. */
		l.nop

/*----------------------------------------------------------------------------*
 *                          boot_itlb_miss_handler                            *
 *----------------------------------------------------------------------------*/
boot_itlb_miss_handler:

	/* 
	 * Mask for ITLB_MR register: - sets V (valid) bit,
	 *                            - sets bits belonging to VPN (15-12)
	 */
	#define ITLB_MR_MASK 0xfffff001

	/* 
	 * Mask for ITLB_TR register: - sets A (access) bit,
	 *                            - sets SXE (superuser execute) bit
	 *                            - sets bits belonging to VPN (15-12)
	 */
	#define ITLB_TR_MASK 0xfffff050

	EXCEPTION_STORE_GPR2
	EXCEPTION_STORE_GPR3
	EXCEPTION_STORE_GPR4
	EXCEPTION_STORE_GPR5
	EXCEPTION_STORE_GPR6

	l.mfspr r4, r0, SPR_EEAR_BASE

	earlyearly:
		CLEAR_GPR(r6)

		l.srli  r3, r4, 13   /* r3 <- r4 / 8192 (sets are relative to page size (8Kb). */

		l.mfspr r6, r0, SPR_IMMUCFGR
		l.andi	r6, r6, SPR_IMMUCFGR_NTS
		l.srli	r6, r6, SPR_IMMUCFGR_NTS_OFF
		l.ori	r5, r0, 0x1
		l.sll	r5, r5, r6 	/* r5 = number IMMU sets from IMMUCFGR. */
		l.addi	r6, r5, -1  /* r6 = nsets mask.                     */
		l.and	r2, r3, r6	/* r2 <- r3 % NSETS_MASK.               */

		l.or    r6, r6, r4
		l.ori   r6, r6, ~(VPN_MASK)         /* r6 <- VPN :VPN .xfff - clear up lo(r6) to 0x**** *fff  */
		l.movhi r5, hi(ITLB_MR_MASK)        /* r5 <- ffff:0000.x000.                                  */
		l.ori   r5, r5, lo(ITLB_MR_MASK)    /* r5 <- ffff:1111.x001 - apply ITLB_MR_MASK.             */
		l.and   r5, r5, r6                  /* r5 <- VPN :VPN .x001 - we have ITLBMR entry.           */
		l.mtspr r2, r5, SPR_ITLBMR_BASE(0)  /* set ITLBMR.                                            */

		/*
		 * Set up ITLB with no translation for EA <= 0x0fffffff
		 *
		 * We need this for head.S mapping (EA = PA). if we move all functions
		 * which run with mmu enabled into entry.S, we might be able to eliminate this.
		 *
		 */
		LOAD_SYMBOL_2_GPR(r6, 0x0fffffff)
		l.sfgeu  r6,r4                     /* flag if r6 >= r4 (if 0xb0ffffff >= EA). */
		l.bf     1f                        /* goto out.                               */
		l.and    r3,r4,r4                  /* delay slot :: 24 <- r4 (if flag==1).    */

		l.add    r3, r0, r4                /* r3 <- PA.                               */

	1:
		l.ori   r3, r3, ~(PPN_MASK)         /* r3 <- PPN :PPN .xfff - clear up lo(r6) to 0x**** *fff. */
		l.movhi r5, hi(ITLB_TR_MASK)        /* r5 <- ffff:0000.x000.                                  */
		l.ori   r5, r5, lo(ITLB_TR_MASK)    /* r5 <- ffff:1111.x050 - apply ITLB_MR_MASK.             */
		l.and   r5, r5, r3                  /* r5 <- PPN :PPN .x050 - we have ITLBTR entry.           */
		l.mtspr r2, r5, SPR_ITLBTR_BASE(0)  /* set ITLBTR.                                            */

		EXCEPTION_LOAD_GPR6
		EXCEPTION_LOAD_GPR5
		EXCEPTION_LOAD_GPR4
		EXCEPTION_LOAD_GPR3
		EXCEPTION_LOAD_GPR2

		l.rfe                               /* SR <- ESR, PC <- EPC. */
		l.nop

/*----------------------------------------------------------------------------*
 *                          dtlb_miss_handler                                 *
 *----------------------------------------------------------------------------*/
dtlb_miss_handler:

	EXCEPTION_STORE_GPR2
	EXCEPTION_STORE_GPR3
	EXCEPTION_STORE_GPR4
	EXCEPTION_STORE_GPR5
	EXCEPTION_STORE_GPR6

	l.mfspr r2, r0, SPR_EEAR_BASE      /* Effective address. */

	LOAD_SYMBOL_2_GPR(r3, curr_proc)   /* Fecthes the page directory by the. */
	LOAD_SYMBOL_2_GPR(r5, KBASE_VIRT)  /* current process. */
	l.sub r3, r3, r5
	l.lwz r3, 0(r3)
	l.sub r3, r3, r5
	l.lwz r3, PROC_CR3(r3)
	
	/* Page directory entry address. */
	l.srli r4, r2, 24
	l.slli r4, r4, 2
	l.add  r4, r4, r3

	/* Page directory entry. */
	l.lwz  r4, 0(r4)
	l.sfeq r4, r0
	l.bf   d_not_pde
	l.nop

	/* PT address.  */
	l.srli r4, r4, 10
	l.slli r4, r4, PAGE_SHIFT
	
	/* PT offset.   */
	l.srli r5, r2, PAGE_SHIFT
	l.andi r5, r5, 0x7FF
	l.slli r5, r5, 2
	l.add  r4, r4, r5      /* PTE address. */

	/* PTE empty. */
	l.lwz  r4, 0(r4)       /* PTE entry.   */
	l.sfeq r4, r0
	l.bf   d_not_pte
	l.nop

	/* PTE not present. */
	l.andi r6, r4, PT_PRESENT
	l.sfeq r6, r0
	l.bf   d_not_pte
	l.nop

	l.srli r5, r4, 10      /* Physical page number. */

	/* DTLB TR Register. */
	l.slli r2, r5, PAGE_SHIFT
	l.andi r4, r4, 0xC0  /* Keep the PPI (2 bits).                    */
	l.ori  r4, r4, 0x305 /* Enable supervidor Read/Write, WBC and CC. */
	l.or   r2, r2, r4

	/* Determine number of DMMU sets and FILL DTLB TR Register. */
	l.mfspr r6, r0, SPR_DMMUCFGR
	l.andi	r6, r6, SPR_DMMUCFGR_NTS
	l.srli	r6, r6, SPR_DMMUCFGR_NTS_OFF
	l.ori	r3, r0, 0x1
	l.sll	r3, r3, r6 	/* r3 = number DMMU sets DMMUCFGR. */
	l.addi	r6, r3, -1  /* r6 = nsets mask.                */
	
	l.mfspr r5, r0, SPR_EEAR_BASE /* Tag. */
	l.srli  r5, r5, PAGE_SHIFT

	l.and	r5, r5, r6	/* offset: & (NUM_TLB_ENTRIES-1).  */
	l.mtspr	r5, r2, SPR_DTLBTR_BASE(0)

	/* Fill DTLB MR Register. */
	l.mfspr r2, r0, SPR_EEAR_BASE
	l.addi	r3, r0, 0xffffe000
	l.and	r3, r2, r3
	l.ori   r3, r3, 1                  /* Valid bit. */
	l.mtspr r5, r3, SPR_DTLBMR_BASE(0)

	EXCEPTION_LOAD_GPR2
	EXCEPTION_LOAD_GPR3
	EXCEPTION_LOAD_GPR4
	EXCEPTION_LOAD_GPR5
	EXCEPTION_LOAD_GPR6
	l.rfe
	l.nop

d_not_pde:
d_not_pte:
	EXCEPTION_LOAD_GPR2
	EXCEPTION_LOAD_GPR3
	EXCEPTION_LOAD_GPR4
	EXCEPTION_LOAD_GPR5
	EXCEPTION_LOAD_GPR6
	
	/* Trigger a dtlb miss page fault. */
	swint_handler 12 _do_page_vfault

/*----------------------------------------------------------------------------*
 *                          itlb_miss_handler                                 *
 *----------------------------------------------------------------------------*/
itlb_miss_handler:

	EXCEPTION_STORE_GPR2
	EXCEPTION_STORE_GPR3
	EXCEPTION_STORE_GPR4
	EXCEPTION_STORE_GPR5
	EXCEPTION_STORE_GPR6

	l.mfspr r2, r0, SPR_EEAR_BASE      /* Effective address. */

	LOAD_SYMBOL_2_GPR(r3, curr_proc)   /* Fecthes the page directory by the. */
	LOAD_SYMBOL_2_GPR(r5, KBASE_VIRT)  /* current process. */
	l.sub r3, r3, r5
	l.lwz r3, 0(r3)
	l.sub r3, r3, r5
	l.lwz r3, PROC_CR3(r3)
	
	/* Page directory entry address. */
	l.srli r4, r2, 24
	l.slli r4, r4, 2
	l.add  r4, r4, r3

	/* Page directory entry. */
	l.lwz  r4, 0(r4)
	l.sfeq r4, r0
	l.bf   i_not_pde
	l.nop

	/* PT address.  */
	l.srli r4, r4, 10
	l.slli r4, r4, PAGE_SHIFT
	
	/* PT offset.   */
	l.srli r5, r2, PAGE_SHIFT
	l.andi r5, r5, 0x7FF
	l.slli r5, r5, 2
	l.add  r4, r4, r5      /* PTE address. */

	/* If PTE empty. */
	l.lwz  r4, 0(r4)       /* PTE entry.   */
	l.sfeq r4, r0
	l.bf   i_not_pte
	l.nop

	/* If PTE not present. */
	l.andi r6, r4, PT_PRESENT
	l.sfeq r6, r0
	l.bf   i_not_pte
	l.nop

	l.srli r5, r4, 10      /* Physical page number. */

	/* ITLB TR Register. */
	l.slli r2, r5, PAGE_SHIFT
	l.ori  r4, r0, 0xC5  /* Enable UXE, SXE, WBC and CC. */
	l.or   r2, r2, r4

	/* Determine number of IMMU sets and FILL ITLB TR Register. */
	l.mfspr r6, r0, SPR_IMMUCFGR
	l.andi	r6, r6, SPR_IMMUCFGR_NTS
	l.srli	r6, r6, SPR_IMMUCFGR_NTS_OFF
	l.ori	r3, r0, 0x1
	l.sll	r3, r3, r6 	/* r3 = number IMMU sets IMMUCFGR. */
	l.addi	r6, r3, -1  /* r6 = nsets mask.                */
	
	l.mfspr r5, r0, SPR_EEAR_BASE /* Tag. */
	l.srli  r5, r5, PAGE_SHIFT

	l.and	r5, r5, r6	/* offset: & (NUM_TLB_ENTRIES-1).  */
	l.mtspr	r5, r2, SPR_ITLBTR_BASE(0)

	/* Fill ITLB MR Register. */
	l.mfspr r2, r0, SPR_EEAR_BASE
	l.addi	r3, r0, 0xffffe000
	l.and	r3, r2, r3
	l.ori   r3, r3, 1                  /* Valid bit. */
	l.mtspr r5, r3, SPR_ITLBMR_BASE(0)

	EXCEPTION_LOAD_GPR2
	EXCEPTION_LOAD_GPR3
	EXCEPTION_LOAD_GPR4
	EXCEPTION_LOAD_GPR5
	EXCEPTION_LOAD_GPR6
	l.rfe
	l.nop

i_not_pde:
i_not_pte:
	EXCEPTION_LOAD_GPR2
	EXCEPTION_LOAD_GPR3
	EXCEPTION_LOAD_GPR4
	EXCEPTION_LOAD_GPR5
	EXCEPTION_LOAD_GPR6
	
	/* Trigger a itlb miss page fault. */
	swint_handler 13 _do_page_vfault

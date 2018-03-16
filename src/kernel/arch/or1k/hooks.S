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
save:
	/* General Purpose registers, except r30 and r9. */
	l.sw GPR2(r1) , r2
	l.sw GPR3(r1) , r3
	l.sw GPR4(r1) , r4
	l.sw GPR5(r1) , r5
	l.sw GPR6(r1) , r6
	l.sw GPR7(r1) , r7
	l.sw GPR8(r1) , r8
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

	/* ECPR. */
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

	l.jr r9
	l.nop

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
	l.sub  r3, r3, r5
	l.lwz  r5, PROC_INTLVL(r3)
	l.addi r5, r5, 1
	l.sw   PROC_INTLVL(r3), r5
    
	/* Save interrupt stack. */
	l.sw PROC_KESP(r3), r1
.endm

/*----------------------------------------------------------------------------*
 *                              exception handler                             *
 *----------------------------------------------------------------------------*/

/* Exceptions handler. */
.macro exception_handler, number
	exception_handler\number:
		EXCEPTION_STORE_GPR3
		EXCEPTION_STORE_GPR5
		enter
		EXCEPTION_LOAD_GPR3
		EXCEPTION_LOAD_GPR5
		
		/* Checks if we are in user or kernel mode. */
		EXCEPTION_T_STORE_GPR30           /* Backup register.          */
		l.mfspr r30, r0,  SPR_ESR_BASE
		l.andi  r30, r30, SPR_SR_SM
		l.sfeqi r30, 0
		l.bnf   2f
		EXCEPTION_T_STORE_SP              /* Delay slot, save SP (r1). */
	
		/* Extracts the Kernel stack. */
	
		/* User mode. */
	1:
		EXCEPTION_STORE_GPR3
		EXCEPTION_STORE_GPR5
		LOAD_SYMBOL_2_GPR(r3, curr_proc)
		LOAD_SYMBOL_2_GPR(r5, KBASE_VIRT)
		l.sub r3, r3, r5
		l.lwz r1, PROC_KESP(r3)

	
		/* Kernel mode. */
	2:
		/* Create new stack frame. */
		l.addi r1, r1, -(INT_FRAME_SIZE)
		l.sub  r1, r1, r5

		/* Save registers. */
		EXCEPTION_LOAD_GPR3
		EXCEPTION_LOAD_GPR5
		EXCEPTION_STORE_GPR9 /* Backup Link Register before the jal. */
		l.jal save
		l.nop
		EXCEPTION_LOAD_GPR9
		l.sw GPR9(r1), r9

		/* Enable MMU. */
		l.mfspr r30, r0, SPR_ESR_BASE
		l.ori	r30, r0, (EXCEPTION_SR)
		l.mtspr	r0, r30, SPR_ESR_BASE

		/* Check if syscall. */
		l.ori   r3, r0, \number
		l.sfeqi r3, 9
		l.bnf 3f

		/* Syscall. */
		LOAD_SYMBOL_2_GPR(r5, syscalls_table)
		l.jal syscall
		l.nop
		l.j 4f
		l.nop

	3:
		/* Not syscall, lets call do_hwint. */
		LOAD_SYMBOL_2_GPR(r5, do_hwint)
		l.jr r5
		l.nop

		/* Return from exception. */
	4:
		l.mfspr r30, r0, SPR_EPCR_BASE
		l.rfe
.endm

/*----------------------------------------------------------------------------*
 *                                  exceptions                                *
 *----------------------------------------------------------------------------*/

.org 0x100
reset_exception:
	exception_handler 0

.org 0x200
bus_exception:
	exception_handler 1

.org 0x300
data_page_fault_exception:
	exception_handler 2

.org 0x400
instruction_page_fault:
	exception_handler 3

.org 0x500
tick_timer_exception:
	exception_handler 4

.org 0x600
alignment_exception:
	exception_handler 5

.org 0x700
illegal_instruction_exception:
	exception_handler 6

.org 0x800
external_interrupt_exception:
	exception_handler 7

.org 0x900
dtlb_miss_exception:
	l.j	boot_dtlb_miss_handler
	l.nop

.org 0xa00
itlb_miss_exception:
	l.j	boot_itlb_miss_handler
	l.nop

.org 0xb00
range_exception:
	exception_handler 8

.org 0xc00
syscall_exception:
	exception_handler 9

.org 0xd00
floating_point_exception:
	exception_handler 10

.org 0xe00
trap_exception:
	exception_handler 11

/*----------------------------------------------------------------------------*
 *                                leave()                                     *
 *----------------------------------------------------------------------------*/

/*
 * Leaves kernel.
 */
leave:

/*----------------------------------------------------------------------------*
 *                                  swint()                                   *
 *----------------------------------------------------------------------------*/


/*----------------------------------------------------------------------------*
 *                                 syscall()                                  *
 *----------------------------------------------------------------------------*/

/*
 * System call hook.
 */
syscall:






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

	/* PT address.  */
	l.srli r4, r4, 10
	l.slli r4, r4, PAGE_SHIFT
	
	/* PT offset.   */
	l.srli r5, r2, PAGE_SHIFT
	l.andi r5, r5, 0x7FF
	l.slli r5, r5, 2
	l.add  r4, r4, r5      /* PTE address. */

	l.lwz  r4, 0(r4)       /* PTE entry.   */
	l.sfeq r4, r0
	l.bf   d_not_pte

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
	
	/* TODO: Trigger a dtlb miss page fault. */
	l.mfspr r2, r0, SPR_EEAR_BASE
	.dtlb_fault_loop:
		l.j .dtlb_fault_loop
		l.nop

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

	/* PT address.  */
	l.srli r4, r4, 10
	l.slli r4, r4, PAGE_SHIFT
	
	/* PT offset.   */
	l.srli r5, r2, PAGE_SHIFT
	l.andi r5, r5, 0x7FF
	l.slli r5, r5, 2
	l.add  r4, r4, r5      /* PTE address. */

	l.lwz  r4, 0(r4)       /* PTE entry.   */
	l.sfeq r4, r0
	l.bf   i_not_pte

	l.srli r5, r4, 10      /* Physical page number. */

	/* ITLB TR Register. */
	l.slli r2, r5, PAGE_SHIFT
	l.andi r4, r4, 0xC0  /* Keep the PPI (2 bits). */
	l.ori  r4, r4, 0x05  /* Enable WBC and CC.     */
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
	
	/* TODO: Trigger a itlb miss page fault. */
	l.mfspr r2, r0, SPR_EEAR_BASE
	.itlb_fault_loop:
		l.j .itlb_fault_loop
		l.nop

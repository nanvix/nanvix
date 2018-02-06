/*
 * Copyright(C) 2017-2017 Davidson Francis <davidsondfgl@gmail.com>
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
#include <or1k/or1k.h>
#include <or1k/asm_defs.h>

.globl dtlb_miss_handler
.globl itlb_miss_handler

.section .exception

/*============================================================================*
 *                                exceptions                                  *
 *============================================================================*/

.org 0x100
reset_exception:
	l.j start
	l.nop

.org 0x200
bus_exception:
	l.nop
	l.nop

.org 0x300
data_page_fault_exception:
	l.nop
	l.nop

.org 0x400
instruction_page_fault:
	l.nop
	l.nop

.org 0x500
tick_timer_exception:
	l.nop
	l.nop

.org 0x600
alignment_exception:
	l.nop
	l.nop

.org 0x700
illegal_instruction_exception:
	l.nop
	l.nop

.org 0x800
external_interrupt_exception:
	l.nop
	l.nop

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
	l.nop
	l.nop

.org 0xc00
syscall_exception:
	l.nop
	l.nop

.org 0xd00
floating_point_exception:
	l.nop
	l.nop

.org 0xe00
trap_exception:
	l.nop
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

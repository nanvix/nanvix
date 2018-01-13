/*
 * Copyright(C) 2011-2017 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2017-2017 Davidson Francis <davidsondfgl@gmail.com>
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

/* Exported symbols. */
.globl start
.globl idle_pgdir

/*============================================================================*
 *                              bootstrap section                             *
 *============================================================================*/

.section .bootstrap

/*----------------------------------------------------------------------------*
 *                                    start                                   *
 *----------------------------------------------------------------------------*/

/*
 * Kernel entry point.
 */
start:

	/* Surpervisor mode. */
	l.ori	r1, r0, 0x1
	l.mtspr	r0, r1, SPR_SR

	/* Stack. */
	l.addi r1, r0, 0x2000
	l.addi r1, r1, -8
	
	/* UART init. */
	l.jal uart_init
	l.nop

	/*
	 * TODO: Build initial RAM disk page table. 
	 */
	LOAD_SYMBOL_2_GPR(r1, initrd_pgtab)

	/* Build kernel and kernel pool page tables. */
	LOAD_SYMBOL_2_GPR(r1, kpool_pgtab)
	l.addi  r1, r1, PAGE_SIZE
	l.addi  r1, r1, -DWORD_SIZE

	LOAD_SYMBOL_2_GPR(r3, 0x003ffc00)
	l.ori  r3, r3, PT_L | PT_WBC | PT_CC

	start.loop1:
		l.sw   0(r1), r3
		l.addi r1, r1, -4
		l.addi r3, r3, -1024
		l.sfgtsi r3, 0
		l.bf start.loop1
		l.nop

	/* Build init page directory. */
	LOAD_SYMBOL_2_GPR(r1, kpgtab)
	l.srli r1, r1, PAGE_SHIFT
	l.slli r1, r1, PT_SHIFT
	l.ori  r1, r1, PT_WBC | PT_CC

	LOAD_SYMBOL_2_GPR(r3, idle_pgdir)
	
	l.sw  PTE_SIZE*0(r3),   r1           /* Kernel code + data at 0x00000000 */
	l.sw  PTE_SIZE*192(r3), r1           /* Kernel code + data at 0xc0000000 */

	LOAD_SYMBOL_2_GPR(r1, kpool_pgtab)   /* Kernel page pool at 0xc1000000   */
	l.srli r1, r1, PAGE_SHIFT
	l.slli r1, r1, PT_SHIFT
	l.ori  r1, r1, PT_WBC | PT_CC
	l.sw  PTE_SIZE*193(r3), r1

	LOAD_SYMBOL_2_GPR(r1, initrd_pgtab)  /* Init RAM disk at 0xc2000000      */
	l.srli r1, r1, PAGE_SHIFT
	l.slli r1, r1, PT_SHIFT
	l.ori  r1, r1, PT_WBC | PT_CC
	l.sw  PTE_SIZE*194(r3), r1

                                         /* Command line data at 0xc3000000  */
	
	/* Flush TLB. */
	l.jal flush_tlb
	l.nop

	/* Enable Instruction Cache. */
	l.jal _ic_enable
	l.nop

	/* Enable Data Cache. */
	l.jal _dc_enable
	l.nop

	/* Enabling IMMU/DMMU */
	l.jal enable_mmu
	l.nop

	/* Install the properly TLB handlers. */
	LOAD_SYMBOL_2_GPR(r1, itlb_miss_handler)
	l.addi r1, r1, -0xa00
	l.srli r1, r1, 2
	l.sw 0xa00(r0), r1

	LOAD_SYMBOL_2_GPR(r1, dtlb_miss_handler)
	l.addi r1, r1, -0x900
	l.srli r1, r1, 2
	l.sw 0x900(r0), r1

	/* Invalidate instruction cache. */
	l.ori   r2, r0, 0x900
	l.mtspr r0, r2, SPR_ICBIR
	l.ori   r2, r0, 0xa00
	l.mtspr r0, r2, SPR_ICBIR

	/* Flush TLB. */
	l.jal flush_tlb
	l.nop

	l.nop
	l.nop
	loop:
		l.j loop
	l.nop

/*----------------------------------------------------------------------------*
 *                                flush_tlb                                   *
 *----------------------------------------------------------------------------*/

flush_tlb:
	LOAD_SYMBOL_2_GPR(r1,SPR_DTLBMR_BASE(0))
	LOAD_SYMBOL_2_GPR(r3,SPR_ITLBMR_BASE(0))
		
	l.addi	r5, r0, 128
	.loop:
		l.mtspr	r1, r0, 0x0
		l.mtspr	r3, r0, 0x0

		l.addi	r1, r1, 1
		l.addi	r3, r3, 1
		l.sfeq	r5, r0
		l.bnf	.loop
		l.addi	r5, r5, -1

	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                enable_mmu                                  *
 *----------------------------------------------------------------------------*/

enable_mmu:
	l.mfspr	r1, r0, SPR_SR
	LOAD_SYMBOL_2_GPR(r3, SPR_SR_DME | SPR_SR_IME)

	l.or	r1, r1, r3
	l.mtspr	r0, r1, SPR_SR
	
	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                _ic_enable                                  *
 *----------------------------------------------------------------------------*/

_ic_enable:
	/* Check if IC present and skip enabling otherwise */
	l.mfspr r24,r0,SPR_UPR
	l.andi  r26,r24,SPR_UPR_ICP
	l.sfeq  r26,r0
	l.bf	9f
	l.nop

	/* Disable IC */
	l.mfspr r6,r0,SPR_SR
	l.addi  r5,r0,-1
	l.xori  r5,r5,SPR_SR_ICE
	l.and   r5,r6,r5
	l.mtspr r0,r5,SPR_SR

	/* 
	 * Establish cache block size
	 * If BS=0, 16;
	 * If BS=1, 32;
	 * r14 contain block size
	 */
	l.mfspr r24,r0,SPR_ICCFGR
	l.andi	r26,r24,SPR_ICCFGR_CBS
	l.srli	r28,r26,7
	l.ori	r30,r0,16
	l.sll	r14,r30,r28

	/*
	 * Establish number of cache sets
	 * r16 contains number of cache sets
	 * r28 contains log(# of cache sets)
	 */
	l.andi  r26,r24,SPR_ICCFGR_NCS
	l.srli 	r28,r26,3
	l.ori   r30,r0,1
	l.sll   r16,r30,r28

	/* Invalidate IC */
	l.addi  r6,r0,0
	l.sll   r5,r14,r28

1:
	l.mtspr r0,r6,SPR_ICBIR
	l.sfne  r6,r5
	l.bf    1b
	l.add   r6,r6,r14

	/* Enable IC */
	l.mfspr r6,r0,SPR_SR
	l.ori   r6,r6,SPR_SR_ICE
	l.mtspr r0,r6,SPR_SR
	l.nop
	l.nop
	l.nop
	l.nop
	l.nop
	l.nop
	l.nop
	l.nop
	l.nop
	l.nop
9:
	l.jr    r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                _dc_enable                                  *
 *----------------------------------------------------------------------------*/

_dc_enable:
	/* Check if DC present and skip enabling otherwise */
	l.mfspr r24,r0,SPR_UPR
	l.andi  r26,r24,SPR_UPR_DCP
	l.sfeq  r26,r0
	l.bf	9f
	l.nop

	/* Disable DC */
	l.mfspr r6,r0,SPR_SR
	l.addi  r5,r0,-1
	l.xori  r5,r5,SPR_SR_DCE
	l.and   r5,r6,r5
	l.mtspr r0,r5,SPR_SR

	/*
	 * Establish cache block size
	 * If BS=0, 16;
	 * If BS=1, 32;
	 * r14 contain block size
	 */
	l.mfspr r24,r0,SPR_DCCFGR
	l.andi	r26,r24,SPR_DCCFGR_CBS
	l.srli	r28,r26,7
	l.ori	r30,r0,16
	l.sll	r14,r30,r28

	/*
	 * Establish number of cache sets
	 * r16 contains number of cache sets
	 * r28 contains log(# of cache sets)
	 */
	l.andi  r26,r24,SPR_DCCFGR_NCS
	l.srli 	r28,r26,3
	l.ori   r30,r0,1
	l.sll   r16,r30,r28

	/* Invalidate DC */
	l.addi  r6,r0,0
	l.sll   r5,r14,r28
1:
	l.mtspr r0,r6,SPR_DCBIR
	l.sfne  r6,r5
	l.bf    1b
	l.add   r6,r6,r14

	/* Enable DC */
	l.mfspr r6,r0,SPR_SR
	l.ori   r6,r6,SPR_SR_DCE
	l.mtspr r0,r6,SPR_SR
9:
	l.jr    r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                   kpgtab                                   *
 *----------------------------------------------------------------------------*/

/* 
 * Kernel page table. 
 */
.align PAGE_SIZE
kpgtab:
	.fill PAGE_SIZE/PTE_SIZE, PTE_SIZE, 0

/*----------------------------------------------------------------------------*
 *                                kpool_pgtab                                 *
 *----------------------------------------------------------------------------*/

/* 
 * Kernel pool page table. 
 */
.align PAGE_SIZE
kpool_pgtab:
	.fill PAGE_SIZE/PTE_SIZE, PTE_SIZE, 0

/*----------------------------------------------------------------------------*
 *                                initrd_pgtab                                *
 *----------------------------------------------------------------------------*/

/* 
 * Initial RAM disk page table.
 */
.align PAGE_SIZE
initrd_pgtab:
	.fill PAGE_SIZE/PTE_SIZE, PTE_SIZE, 0

/*----------------------------------------------------------------------------*
 *                                  idle_pgdir                                *
 *----------------------------------------------------------------------------*/

/* 
 * init page directory.
 */
.align PAGE_SIZE
idle_pgdir:
	.fill 256, PTE_SIZE, 0

/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2017-2017 Clement Rouquier <clementrouquier@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

/* Must come first. */
#define _ASM_FILE_

#include <i386/i386.h>
#include <nanvix/config.h>
#include <nanvix/mboot.h>
#include <nanvix/mm.h>

/* Multiboot flags. */
#define MBOOT_FLAGS (MBOOT_PAGE_ALIGN | MBOOT_MEMORY_INFO)

/* Command-line offset */
#define MBOOT_KPARAM_OFFSET 0x00000008

/* Exported symbols. */
.globl start
.globl idle_pgdir

/*============================================================================*
 *                              bootstrap section                             *
 *============================================================================*/

.section .bootstrap

/*----------------------------------------------------------------------------*
 *                               multiboot header                             *
 *----------------------------------------------------------------------------*/

/*
 * Grub multiboot header.
 */
.align 4
mboot_header:
    .long  MBOOT_MAGIC                  /* Magic number.              */
    .long  MBOOT_FLAGS                  /* Flags.                     */
    .long  -(MBOOT_MAGIC + MBOOT_FLAGS) /* Checksum.                  */
    .long  mboot_header                 /* Pointer to this structure. */

/*----------------------------------------------------------------------------*
 *                                    start                                   *
 *----------------------------------------------------------------------------*/

/*
 * Kernel entry point.
 */
start:	
	cmpl $1, 20(%ebx)
	jne halt
	
	/* Retrieve initrd location. */
	movl 24(%ebx), %edx
	movl (%edx), %edx

	/* Retrieve command-line passed by GRUB. */
	movl $cmdline, %edi
	movl 16(%ebx), %ecx
	addl $MBOOT_KPARAM_OFFSET, %ecx
	jmp bottom
	top:
		addl $4, %ecx 
		stosl
	bottom:
		movl (%ecx), %eax
		cmpl $0,%eax
		jne top 
		
	/* Put back initrd into eax. */
	movl %edx,%eax

	/* Build initial RAM disk page table. */
	movl $initrd_pgtab, %edi
	addl $7, %eax	
	movl %eax, %ebx
	addl $INITRD_SIZE, %ebx
	cld
	start.loop0:
		cmpl %eax, %ebx
		je start.endloop0
		stosl
		addl $PAGE_SIZE, %eax
		jmp start.loop0
	start.endloop0:

	/* Build kernel page tables. */
	movl $kpgtab, %edi
	movl $0x00000000 + 7, %eax
	start.loop1:
		stosl
		addl $PAGE_SIZE,   %eax
		cmpl $kpool_pgtab, %edi
		jl start.loop1

	/* Build kernel pool tables. */
	movl $kpool_pgtab, %edi
	movl $0x01000000 + 7, %eax
	start.loop2:
		stosl
		addl $PAGE_SIZE,    %eax
		cmpl $initrd_pgtab, %edi
		jl start.loop2

	/* Build init page directory. */
	movl $kpgtab + 3, idle_pgdir + PTE_SIZE*0         /* Kernel code + data at 0x00000000 */
	movl $kpgtab + 3, idle_pgdir + PTE_SIZE*768       /* Kernel code + data at 0xc0000000 */
	movl $kpool_pgtab + 3, idle_pgdir + PTE_SIZE*772  /* Kernel page pool at 0xc1000000   */
	movl $initrd_pgtab + 3, idle_pgdir + PTE_SIZE*776 /* Init RAM disk at 0xc2000000      */
	movl $cmdline + 3, idle_pgdir + PTE_SIZE*780      /* Command line data at 0xc3000000  */
	
	/* Enable paging. */
	movl $idle_pgdir, %eax
	movl %eax, %cr3
	movl %cr0, %eax
	orl $0x80000000, %eax
	movl %eax, %cr0

	/* Setup stack. */
	movl $idle_kstack + PAGE_SIZE - DWORD_SIZE, %ebp
	movl $idle_kstack + PAGE_SIZE - DWORD_SIZE, %esp

	/* Pass command line to the kernel */
	push $cmdline
	
	call setup
	
	call kmain
	
	cli
	
	halt:
		hlt
		jmp halt

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
	.fill PAGE_SIZE/PTE_SIZE, PTE_SIZE, 0

/*----------------------------------------------------------------------------*
 *                                  cmdline                                *
 *----------------------------------------------------------------------------*/

/* 
 * Command line parameters.
 */
.align PAGE_SIZE
cmdline:
	.fill PAGE_SIZE/PTE_SIZE, PTE_SIZE, 0


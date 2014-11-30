/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * boot.S - Kernel bootstrap
 */

/* Must come first. */
#define _ASM_FILE_

#include <i386/i386.h>
#include <nanvix/config.h>
#include <nanvix/mboot.h>
#include <nanvix/mm.h>

/* Multiboot flags. */
#define MBOOT_FLAGS (MBOOT_PAGE_ALIGN | MBOOT_MEMORY_INFO)

/* Exported symbols. */
.globl start
.globl init_pgdir

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
	movl 24(%ebx), %eax
	movl (%eax), %eax
	
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

	/* Build kernel and kernel pool page tables. */
	movl $kpool_pgtab + PAGE_SIZE - DWORD_SIZE, %edi
	movl $0x07ff000 + 7, %eax
	std
	start.loop1:
		stosl
		subl $PAGE_SIZE, %eax
		jge start.loop1
	cld

	/* Build init page directory. */
	movl $kpgtab + 3, init_pgdir + PTE_SIZE*0         /* at: 0x00000000 */
	movl $kpgtab + 3, init_pgdir + PTE_SIZE*768       /* at: 0xc0000000 */
	movl $kpool_pgtab + 3, init_pgdir + PTE_SIZE*769  /* at: 0xc0400000 */
	movl $initrd_pgtab + 3, init_pgdir + PTE_SIZE*770 /* at: 0xc0800000 */
	
	/* Enable paging. */
	movl $init_pgdir, %eax
	movl %eax, %cr3
	movl %cr0, %eax
	orl $0x80000000, %eax
	movl %eax, %cr0
	
	/* Setup stack. */
	movl $init_kstack + PAGE_SIZE - DWORD_SIZE, %ebp
	movl $init_kstack + PAGE_SIZE - DWORD_SIZE, %esp
	
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
 *                                  init_pgdir                                *
 *----------------------------------------------------------------------------*/

/* 
 * init page directory.
 */
.align PAGE_SIZE
init_pgdir:
	.fill PAGE_SIZE/PTE_SIZE, PTE_SIZE, 0

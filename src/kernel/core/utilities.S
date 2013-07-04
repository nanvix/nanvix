/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * utilities.S - Low-level utilities
 */

/* Must come first. */
#define _ASM_FILE_

#include <i386/i386.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>

/* Exported symbols. */
.globl gdt_flush
.globl idt_flush
.globl tss_flush
.globl enable_interrupts
.globl disable_interrupts
.globl halt
.globl physcpy
.globl switch_to

/*----------------------------------------------------------------------------*
 *                                 gdt_flush                                  *
 *----------------------------------------------------------------------------*/

/*
 * Flushes the GDT.
 */
gdt_flush:
	/* Load GDT. */
	mov 4(%esp), %eax
    lgdt (%eax)
    
    /* Reload code segment. */
    ljmp $KERNEL_CS, $reload_cs
    reload_cs :

    /* Realod data segment. */
    movw $KERNEL_DS, %ax
    movw %ax, %ds

    ret

/*----------------------------------------------------------------------------*
 *                                 tss_flush                                  *
 *----------------------------------------------------------------------------*/

/*
 * Flushes the TSS.
 */
tss_flush:
	/* Load TSS.*/
	movl $TSS, %eax
	ltr %ax
	ret

/*----------------------------------------------------------------------------*
 *                                 idt_flush                                  *
 *----------------------------------------------------------------------------*/

/*
 * Flushes the IDT.
 */
idt_flush:
	/* Load iDT. */
	mov 4(%esp), %eax
    lidt (%eax)
    ret

/*----------------------------------------------------------------------------*
 *                            enable_interrupts()                             *
 *----------------------------------------------------------------------------*/
 
/*
 * Enables all hardware interrupts.
 */
enable_interrupts:
	sti
	ret

/*----------------------------------------------------------------------------*
 *                            disable_interrupts()                            *
 *----------------------------------------------------------------------------*/
 
/*
 * Disables all hardware interrupts.
 */
disable_interrupts:
	cli
	ret

/*----------------------------------------------------------------------------*
 *                                   halt()                                   *
 *----------------------------------------------------------------------------*/
 
/*
 * Halts the processor.
 */
halt:
	hlt
	ret

/*----------------------------------------------------------------------------*
 *                                 physcpy()                                  *
 *----------------------------------------------------------------------------*/

/*
 * Physical memory copy.
 */
physcpy:
	cli
	
	/* Get parameters. */
	movl  8(%esp), %ecx
	movl 12(%esp), %esi
	movl 16(%esp), %edi

	/* Jump to lower-half kernel*/
	movl $lower_kernel, %eax
	subl $KBASE_VIRT, %eax
	jmp *%eax
	lower_kernel:
 
  	/* Disable paging. */
  	movl %cr0, %eax
  	andl $0x80000000 - 1, %eax
  	movl %eax, %cr0
  	
  	std; stosb; rep           
  
  	/* Re-enable paging. */
	movl %cr0, %eax
	orl $0x80000000, %eax
	movl %eax, %cr0
  
	/* Come back to higher-half kernel. */
	movl $higher_kernel, %eax
	jmp *%eax
	higher_kernel:
  
	sti
    ret

/*----------------------------------------------------------------------------*
 *                                switch_to()                                 *
 *----------------------------------------------------------------------------*/

/*
 * Switches execution to other process.
 */
switch_to:
	movl 4(%esp), %ecx

	/* Save process context. */
	movl curr_proc, %eax
	pushfl
	pushl %ebx
	pushl %ebp
	movl %esp, PROC_KESP(%eax)
	
	/* Switch processes. */
	movl %ecx, curr_proc

	/* Load process address space. */
	movl PROC_CR3(%ecx), %eax
	movl %eax, %cr3
	movl PROC_KSTACK(%ecx), %eax
	addl $PAGE_SIZE - DWORD_SIZE, %eax
	movl %eax, TSS_ESP + tss
	
	/* Load process context. */
	movl PROC_KESP(%ecx), %esp
	popl %ebp
	popl %ecx
	popfl
	
	ret

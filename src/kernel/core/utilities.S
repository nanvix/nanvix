/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * utilities.S - Low-level utilities
 */

/* Must come first. */
#define _ASM_FILE_

#include <i386/i386.h>
#include <nanvix/int.h>
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
.globl ksetjmp
.globl klongjmp
.globl cpulvl

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
	
	pushl %esi
	pushl %edi
	
	/* Get parameters. */
	movl 16(%esp), %ecx
	movl 20(%esp), %esi
	movl 24(%esp), %edi

	/* Jump to lower-half kernel*/
	cli
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
  
	popl %edi
	popl %esi
  
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
	pushl PROC_KESP(%eax)
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
	
	movl PROC_FLAGS(%ecx), %eax
	andl $PROC_NEW, %eax
	jnz fork_return
	
	popl PROC_KESP(%ecx)
	
	popl %ebp
	popl %ecx
	popfl
	
	ret

/*
 * Returns from fork.
 */
fork_return:
	notl %eax
	andl %eax, PROC_FLAGS(%ecx)
	movl $0, EAX(%esp)
	jmp leave

/*----------------------------------------------------------------------------*
 *                                  ksetjmp()                                 *
 *----------------------------------------------------------------------------*/

/*
 * Sets a jump point for a non-local goto.
 */
ksetjmp:
	xchg %bx, %bx
	movl curr_proc, %eax
	movl 4(%esp), %ecx
	movl 0(%esp), %edx
	movl %ebx, JMP_BUF_EBX(%ecx)
	movl %esi, JMP_BUF_ESI(%ecx)
	movl %edi, JMP_BUF_EDI(%ecx)
	movl %edx, JMP_BUF_EIP(%ecx)
	pushfl
	popl %edx
	movl %edx, JMP_BUF_EFLAGS(%ecx)
	movl %ebp, JMP_BUF_EBP(%ecx)
	movl %esp, JMP_BUF_ESP(%ecx)
	movl PROC_INTLVL(%eax), %edx
	movl %edx, JMP_BUF_INTLVL(%ecx)
	movl PROC_KESP(%eax), %edx
	movl %edx, JMP_BUF_KESP(%ecx)
	xorl %eax, %eax
	ret

/*----------------------------------------------------------------------------*
 *                                  klongjmp()                                *
 *----------------------------------------------------------------------------*/

/*
 * Performs a non-local goto.
 */
klongjmp:
	xchg %bx, %bx
	movl 8(%esp), %eax
	movl 4(%esp), %ecx
	movl curr_proc, %edx
	movl JMP_BUF_EBX(%ecx), %ebx
	movl JMP_BUF_ESI(%ecx), %esi
	movl JMP_BUF_EDI(%ecx), %edi
	pushl JMP_BUF_EFLAGS(%ecx)
	popfl
	movl JMP_BUF_EBP(%ecx), %ebp
	movl JMP_BUF_ESP(%ecx), %esp
	pushl JMP_BUF_INTLVL(%ecx)
	popl PROC_INTLVL(%edx)
	pushl JMP_BUF_KESP(%ecx)
	popl  PROC_KESP(%edx)
	jmpl *JMP_BUF_EIP(%ecx)

/*----------------------------------------------------------------------------*
 *                                   cpulvl()                                 *
 *----------------------------------------------------------------------------*/

/*
 * Changes the CPU level.
 */
cpulvl:
	movl 4(%esp), %ecx
	xorl %eax, %eax
	inb $PIC_DATA_SLAVE, %al
	movb %al, %ah
	inb $PIC_DATA_MASTER, %al
	xchg %eax, %ecx
	outb %al, $PIC_DATA_MASTER
	movb %ah, %al
	outb %al, $PIC_DATA_SLAVE
	xchg %eax, %ecx
	ret

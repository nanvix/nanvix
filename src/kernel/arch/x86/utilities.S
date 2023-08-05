/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2016 Davidson Francis <davidsondfgl@gmail.com>
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
#include <i386/int.h>
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

/* Imported symbols. */
.globl processor_reload

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
 *                                 tlb_flush                                  *
 *----------------------------------------------------------------------------*/

/*
 * Flushes the TLB.
 */
tlb_flush:
	movl %cr3, %eax
	movl %eax, %cr3
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
	nop
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
	movl 12(%esp), %edi
	movl 16(%esp), %esi
	movl 20(%esp), %ecx

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

/*
 * Copy memory from a page to another.
 * We cannot use nice instructions such as
 * movsb because we would use segment registers
 * and therefore the GDT, which is only accessible
 * when paging is enabled.
 */
physcpy.loop:
	movl (%esi), %eax
	movl %eax, (%edi)
	addl $4, %esi
	addl $4, %edi
	subl $4, %ecx
	jnz physcpy.loop

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
	pushl %esi
	pushl %edi
	pushl %ebp
	pushl PROC_KESP(%eax)
	movl %esp, PROC_KESP(%eax)
	fnsave PROC_FSS(%eax)

	/* Switch processes. */
	movl %ecx, curr_proc

	/* Load process address space. */
	movl PROC_CR3(%ecx), %eax
	movl %eax, %cr3
	movl PROC_KSTACK(%ecx), %eax
	addl $PAGE_SIZE - DWORD_SIZE, %eax
	movl %eax, TSS_ESP0 + tss

	/* Load process context. */
	movl PROC_KESP(%ecx), %esp
	frstor PROC_FSS(%ecx)

	pushl %ecx
	call processor_reload
	popl %ecx

	btrl $PROC_NEW, PROC_FLAGS(%ecx)
	jc fork_return

	popl PROC_KESP(%ecx)

	popl %ebp
	popl %edi
	popl %esi
	popl %ebx
	popfl

	ret

/*
 * Returns from fork.
 */
fork_return:
	movl $0, EAX(%esp)
	jmp leave

/*----------------------------------------------------------------------------*
 *                                 user_mode()                                *
 *----------------------------------------------------------------------------*/

/*
 * Switches to user mode.
 */
user_mode:
	movl 4(%esp), %ecx
	movl 8(%esp), %edx

	/* Enter critical region. */
	cli

	/* Set interrupt level to "user level". */
	movl curr_proc, %ebx
    movl $0, PROC_INTLVL(%ebx)

	/* Load data segment selector. */
	movw $USER_DS, %ax
	movw %ax, %ds

	/* Build fake interrupt stack. */
	pushl $USER_DS
	pushl %edx
	pushfl
	orl $0x200, (%esp)
	pushl $USER_CS
	pushl %ecx
	iret

/*----------------------------------------------------------------------------*
 *                                 fpu_init()                                *
 *----------------------------------------------------------------------------*/

/*
 * Initializes the FPU.
 */
fpu_init:
	fninit
	movl curr_proc, %eax
	fnsave  PROC_FSS(%eax)
	ret

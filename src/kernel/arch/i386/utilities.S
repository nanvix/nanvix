/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2017 Davidson Francis <davidsondfgl@gmail.com>
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
.globl setup_interrupts
.globl enable_interrupts
.globl disable_interrupts
.globl halt
.globl physcpy
.globl switch_to
.globl user_mode
.globl forge_stack
.globl save_ipi_context
.globl spin_init
.globl spin_lock
.globl spin_unlock
.globl fpu_init
.globl pmc_init
.globl read_pmc
.globl write_msr

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
 *                            setup_interrupts()                              *
 *----------------------------------------------------------------------------*/
 
/*
 * Enables and unmask all hardware interrupts.
 */
setup_interrupts:
	nop
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
	movl 8(%esp), %edx

	/* Save process context. */
	lea  cpus, %eax
	movl PERCORE_CURRTHREAD(%eax), %eax
	pushfl
	pushl %ebx
	pushl %esi
	pushl %edi
	pushl %ebp
	pushl THRD_KESP(%eax)
	movl %esp, THRD_KESP(%eax)
	fnsave THRD_FSS(%eax)
	
	/* Switch threads. */
	lea  cpus, %eax
	movl %edx, PERCORE_CURRTHREAD(%eax)

	/* Load process address space. */
	movl PROC_CR3(%ecx), %eax
	movl %eax, %cr3
	movl THRD_KSTACK(%edx), %eax
	addl $PAGE_SIZE - DWORD_SIZE, %eax
	movl %eax, TSS_ESP0 + tss
	
	/* Load process context. */
	movl THRD_KESP(%edx), %esp
	frstor THRD_FSS(%edx)

	push %ecx
	push %edx
	call processor_reload
	pop %edx
	pop %ecx
	
	/* New process?. */
	btrl $PROC_NEW, PROC_FLAGS(%ecx)
	jc fork_return
	
	/* New thread?. */
	btrl $THRD_NEW, THRD_FLAGS(%edx)
	jc pthread_create_return
	
	popl THRD_KESP(%edx)
	
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
	
/*
 * Returns from pthread_create.
 */
pthread_create_return:
	/* Decrement interrupt level. */
	lea  cpus, %ebx
	movl PERCORE_CURRTHREAD(%ebx), %ebx
	decl THRD_INTLVL(%ebx)
	
	movl $0, EAX(%esp)
	addl $4, %esp
	jmp leave.out

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
	lea  cpus, %eax
	movl PERCORE_CURRTHREAD(%eax), %eax
	movl $1, THRD_INTLVL(%eax)
	
	/* Reset flags. */
	movl $0, THRD_FLAGS(%eax)

	/* Retrieves the intstack. */
	movl THRD_INTSTACK(%eax), %eax
	
	/* General Purpose registers. */
	movl $0, EAX(%eax)
	movl $0, EBX(%eax)
	movl $0, ECX(%eax)
	movl $0, EDX(%eax)
	movl $0, ESI(%eax)
	movl $0, EDI(%eax)
	
	/* Segments. */
	movl $USER_CS, CS(%eax)
	movl $USER_DS, DS(%eax)
	movl $USER_DS, ES(%eax)
	movl $USER_DS, FS(%eax)
	movl $USER_DS, GS(%eax)
	movl $USER_DS, SS(%eax)
	
	/* Stack. */
	movl %edx, USERESP(%eax)
	movl %edx, EBP(%eax)
	
	/* EFLAGS. */
	pushfl
	orl  $0x200, (%esp)
	popl %edx
	movl %edx, EFLAGS(%eax)
	
	/* EIP. */
	movl %ecx, EIP(%eax)
	
	ret

/*----------------------------------------------------------------------------*
 *                                  forge_stack()                             *
 *----------------------------------------------------------------------------*/

/*
 * Forge the expected stack for a new thread.
 */
forge_stack:
	/* Preserve registers. */
	pushl %ebx
	pushl %esi
	
	/* Parameters. */
	movl 12(%esp), %eax
	movl 16(%esp), %ebx
	movl 20(%esp), %ecx
	movl 24(%esp), %edx
	movl 28(%esp), %esi			
	
	/* Adjust kern_sp using kstack page */
	addl $PAGE_SIZE,      %eax
	subl $DWORD_SIZE,     %eax
	subl $INT_FRAME_SIZE, %eax
	pushl %eax /* kern_sp pointer will be returned */
	
	/* General Purpose registers. */
	movl $0, EAX(%eax)
	movl $0, EBX(%eax)
	movl $0, ECX(%eax)
	movl $0, EDX(%eax)
	movl $0, ESI(%eax)
	movl $0, EDI(%eax)
	
	/* Segments. */
	movl $USER_CS, CS(%eax)	
	movl $USER_DS, DS(%eax)
	movl $USER_DS, ES(%eax)
	movl $USER_DS, FS(%eax)
	movl $USER_DS, GS(%eax)
	movl $USER_DS, SS(%eax)
	
	/* Stack. */
	subl $8,   %ecx
	movl %ecx, USERESP(%eax)
	movl %ecx, EBP(%eax)

	/* start_thread parameters. */
	movl %edx, 8(%ecx)
	movl %ebx, 4(%ecx)
	
	/* EFLAGS. */
	pushfl
	orl   $0x200, (%esp)
	popl  %edx
	movl  %edx, EFLAGS(%eax)
	
	/* EIP. */
	movl %esi, EIP(%eax)
		
	popl %eax
	popl %esi
	popl %ebx
	ret
	
/*----------------------------------------------------------------------------*
 *                              save_ipi_context()                            *
 *----------------------------------------------------------------------------*/

/*
 * Saves the current context for the master core into the
 * current thread IPI stack.
 */
save_ipi_context:
	nop
	ret
	
/*----------------------------------------------------------------------------*
 *                                   spin_init                                *
 *----------------------------------------------------------------------------*/

/*
 * Initializes the spin-lock with a given lock.
 */
spin_init:
	nop
	ret

/*----------------------------------------------------------------------------*
 *                                   spin_lock                                *
 *----------------------------------------------------------------------------*/

/*
 * Spin-locks until the lock becomes 0, and so, locks again.
 * lock = 0, unlocked
 * lock = 1, locked
 */
spin_lock:
	nop
1:
	ret

/*----------------------------------------------------------------------------*
 *                                  spin_unlock                               *
 *----------------------------------------------------------------------------*/

/*
 * Unlocks the spin-lock.
 */
spin_unlock:
	nop
	ret

/*----------------------------------------------------------------------------*
 *                                 fpu_init()                                 *
 *----------------------------------------------------------------------------*/

/*
 * Initializes the FPU.
 */
fpu_init:
	fninit
	lea threadtab, %eax
	fnsave THRD_FSS(%eax)
	ret

/*----------------------------------------------------------------------------*
 *                                  pmc_init()                                *
 *----------------------------------------------------------------------------*/

/*
 * Reset the PMC counters to a known state.
 */
pmc_init:
	pushl %eax
	pushl %ecx
	pushl %edx

	/* Clear the global ctrl. */
	xor %edx, %edx
	xor %eax, %eax
	mov $IA32_PERF_GLOBAL_CTRL, %ecx
	wrmsr

	/* Clear the counter event. */
	mov $IA32_PERFEVTSELx, %ecx
	wrmsr

	inc %ecx
	wrmsr

	/* Clear the counter value. */
	mov $IA32_PMCx, %ecx
	wrmsr

	inc %ecx
	wrmsr

	/* Clear PMC0 and PMC1 overflow status. */
	mov $IA32_PERF_GLOBAL_OVF_CTRL, %ecx
	mov $IA32_PMC0, %eax
	or  $IA32_PMC1, %eax
	wrmsr

	popl %edx
	popl %ecx
	popl %eax
	ret

/*----------------------------------------------------------------------------*
 *                                  read_pmc()                                *
 *----------------------------------------------------------------------------*/

/*
 * Reset a specified performance counter.
 */
read_pmc:
	pushl %ebx
	pushl %ecx

	xorl %eax, %eax
	cpuid

	movl 12(%esp), %ecx
	rdpmc

	popl %ecx
	popl %ebx
	ret

/*----------------------------------------------------------------------------*
 *                                  read_msr()                                *
 *----------------------------------------------------------------------------*/

/*
 * Reads from an specified MSR register.
 */
read_msr:
	pushl %ecx

	movl 8(%esp), %ecx
	rdmsr

	popl %ecx
	ret

/*----------------------------------------------------------------------------*
 *                                  write_msr()                               *
 *----------------------------------------------------------------------------*/

/*
 * Writes to an specified MSR register.
 */
write_msr:
	pushl %eax
	pushl %ecx
	pushl %edx

	movl 16(%esp), %ecx
	movl 20(%esp), %eax
	movl 24(%esp), %edx
	wrmsr

	popl %edx
	popl %ecx
	popl %eax
	ret


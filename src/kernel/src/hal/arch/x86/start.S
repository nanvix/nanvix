/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*============================================================================*
 * Constants                                                                  *
 *============================================================================*/

/* Page size. */
#define PAGE_SIZE 4096

/* KSTACK_SIZE is defined via -DKSTACK_SIZE=<value> from build.rs. */

/* KREDZONE_SIZE is defined via -DKREDZONE_SIZE=<value> from build.rs. */

/* KSTACK_GUARD_PATTERN is defined via -DKSTACK_GUARD_PATTERN=<value> from build.rs. */

#include "asm_defs.h"

/*============================================================================*
 * Imported Symbols                                                           *
 *============================================================================*/

.extern kmain
.extern __BSS_START
.extern __BSS_END
.extern do_ap_start
.extern EXCP_STACK_GUARD

/*============================================================================*
 * Bootstrap Section                                                          *
 *============================================================================*/

.section .bootstrap,"ax",@progbits

/*----------------------------------------------------------------------------*
 * _do_start()                                                                *
 *----------------------------------------------------------------------------*/

/*
 * Entry point.
 */
.align 4
.globl _do_start2
_do_start2:
    mov $0x10, %dx
    mov %dx, %ds
    mov %dx, %es
    mov %dx, %fs
    mov %dx, %gs
    mov %dx, %ss

    /*
     * EAX and EBX registers store boot information:
     *  - EAX: Magic value.
     *  - EBX: Address of boot information structure.
     */

    /*
     * Fill BSS section with zeros.
     * We cannot do this later, because the stack is in the BSS.
     * We preserve values of EAX and EBX, as they store boot information.
     */
    movl %eax, %edx          /* Save value of EAX.                 */
    movl $__BSS_START, %edi  /* Start address of BSS section.      */
    movl $__BSS_END, %ecx    /* End address of BSS section.        */
    subl %edi, %ecx          /* Calculate the size of BSS section. */
    xorl %eax, %eax          /* Zero out EAX.                      */
    cld                      /* Increment EDI register.            */
    rep stosb                /* Fill BSS section with zeros.       */

    /*
     * Fill the boot stack guard page with a watermark pattern.
     * If a stack overflow occurs, this pattern will be overwritten,
     * allowing runtime detection of stack corruption.
     *
     * TODO (#1432): Investigate whether moving the kstack to the .data section
     * would allow filling the guard page at compile time instead of runtime.
     */
    movl $kstack_guard, %edi         /* Start of guard page.       */
    movl $(PAGE_SIZE / 4), %ecx      /* Number of 32-bit words.    */
    movl $KSTACK_GUARD_PATTERN, %eax /* Watermark pattern.         */
    rep stosl                        /* Fill guard page.           */

    movl %edx, %eax          /* Restore EAX register.              */

    /* Reset stack. */
    movl $kstack, %esp /* Stack pointer. */
    movl %esp, %ebp    /* Frame pointer. */

    /*
     * Initialize the dynamic stack overflow guard for the boot stack.
     * CONTEXT_HW_SIZE (24) accounts for the maximum hardware-pushed exception
     * frame: EIP + CS + EFLAGS + ESP + SS + error code.
     *
     * No LOCK prefix needed: aligned 32-bit stores are naturally atomic on
     * x86, and at this point only the BSP is running with interrupts disabled.
     */
    movl $(kstack_guard + CONTEXT_HW_SIZE), EXCP_STACK_GUARD

    /* Save boot information on the stack. */
    push %ebx
    push %eax

    /*
     * Clear all general purpose registers,
     * so we have a deterministic startup.
     */
	xorl %eax, %eax
    xorl %ebx, %ebx
    xorl %ecx, %ecx
    xorl %edx, %edx
    xorl %esi, %esi
    xorl %edi, %edi

    /* Call kernel main function. */
    push %esp
    call kmain
    addl $4, %esp

    /* Cleanup boot information. */
    addl $8, %esp

    /* Halt execution. */
	1: hlt
       jmp 1b

.align 4
.globl _do_ap_start
_do_ap_start:
    mov $0x10, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss

    /*
     * Clear all general purpose registers,
     * so we have a deterministic startup.
     */
	xorl %eax, %eax
    xorl %ebx, %ebx
    xorl %ecx, %ecx
    xorl %edx, %edx
    xorl %esi, %esi
    xorl %edi, %edi

    /* Setup stack. */
    movl (kredzone), %esp /* Stack pointer. */
    movl %esp, %ebp        /* Frame pointer. */

    /*
     * Get local APIC ID.
     * TODO: move this to Rust.
     */
    movl  $1, %eax
    cpuid
    shrl  $24, %ebx

    /* Call kernel main function. */
    pushl %ebx
    call do_ap_start
    addl $4, %esp

    /* Halt execution. */
	1: hlt
       jmp 1b

/*============================================================================*
 * BSS Section                                                                *
 *============================================================================*/

.section .bss

/*----------------------------------------------------------------------------*
 * kstack                                                                     *
 *----------------------------------------------------------------------------*/

.align PAGE_SIZE
.globl kstack_guard
kstack_guard:
.space KSTACK_SIZE
.globl kstack
kstack:

/*----------------------------------------------------------------------------*
 * Kernel Red Zone                                                            *
 *----------------------------------------------------------------------------*/

.globl kredzone
kredzone:
.space KREDZONE_SIZE

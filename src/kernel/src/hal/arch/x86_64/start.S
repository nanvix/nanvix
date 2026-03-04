/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*============================================================================*
 * Constants                                                                  *
 *============================================================================*/

/* Page size. */
#define PAGE_SIZE 4096

/* Word size in 64-bit mode. */
#define WORD_SIZE 8

/* KSTACK_SIZE is defined via -DKSTACK_SIZE=<value> from build.rs. */

/* KREDZONE_SIZE is defined via -DKREDZONE_SIZE=<value> from build.rs. */

/* KSTACK_GUARD_PATTERN is defined via -DKSTACK_GUARD_PATTERN=<value> from build.rs. */

/*============================================================================*
 * Imported Symbols                                                           *
 *============================================================================*/

.extern kmain
.extern __BSS_START
.extern __BSS_END

/*============================================================================*
 * Bootstrap Section                                                          *
 *============================================================================*/

.section .bootstrap,"ax",@progbits

/*----------------------------------------------------------------------------*
 * _do_start()                                                                *
 *----------------------------------------------------------------------------*/

/*
 * Entry point for long mode.
 *
 * The VMM (UserVM/KVM) has already set up:
 *   - Long mode (CR4.PAE, EFER.LME, CR0.PG)
 *   - GDT with 64-bit code and data segments
 *   - Identity-mapped page tables (PML4)
 *   - CS pointing to a 64-bit code segment
 *
 * We arrive here in 64-bit mode with a flat address space.
 */
.align 16
.globl _do_start
_do_start:
.globl _do_start2
_do_start2:
    /* Set up data segment registers. */
    mov $0x10, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss

    /*
     * RDI and RSI may carry boot information from the VMM.
     * Save them before clearing BSS (which uses RDI).
     */
    mov %rdi, %r12
    mov %rsi, %r13

    /*
     * Fill BSS section with zeros.
     * We cannot do this later, because the stack is in the BSS.
     */
    lea __BSS_START(%rip), %rdi
    lea __BSS_END(%rip), %rcx
    sub %rdi, %rcx
    xor %eax, %eax
    cld
    rep stosb

    /*
     * Fill the boot stack guard page with a watermark pattern.
     */
    lea kstack_guard(%rip), %rdi
    mov $(PAGE_SIZE / 8), %ecx
    movabs $KSTACK_GUARD_PATTERN, %rax
    /* Replicate 32-bit pattern into full 64-bit qword. */
    mov %eax, %edx
    shl $32, %rax
    or  %rdx, %rax
    rep stosq

    /* Restore boot information. */
    mov %r12, %rdi
    mov %r13, %rsi

    /* Set up stack. */
    lea kstack(%rip), %rsp
    mov %rsp, %rbp

    /*
     * Clear all general purpose registers for a deterministic startup.
     * Preserve RDI, RSI (boot info) on the stack.
     */
    push %rdi
    push %rsi
    xor %eax, %eax
    xor %ebx, %ebx
    xor %ecx, %ecx
    xor %edx, %edx
    xor %r8d, %r8d
    xor %r9d, %r9d
    xor %r10d, %r10d
    xor %r11d, %r11d
    xor %r12d, %r12d
    xor %r13d, %r13d
    xor %r14d, %r14d
    xor %r15d, %r15d

    /* Restore boot info into RDI (first argument to kmain). */
    pop %rsi
    pop %rdi

    /* Call kernel main function.
     * RDI = boot info pointer (System V AMD64 ABI first argument). */
    mov %rsp, %rdi
    call kmain

    /* Halt execution. */
1:  hlt
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

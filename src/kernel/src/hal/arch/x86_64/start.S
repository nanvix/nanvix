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
    /*
     * The VMM passes boot information in RAX and RBX:
     *   RAX = boot magic (0x0c00ffee)
     *   RBX = encoded initrd base/size
     * Save them BEFORE touching AX for segment setup.
     */
    mov %rax, %r12
    mov %rbx, %r13

    /* Set up data segment registers. */
    mov $0x10, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss

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

    /* Set up stack. */
    lea kstack(%rip), %rsp
    mov %rsp, %rbp

    /*
     * Build KernelArguments on the stack.
     * Layout (16 bytes, matching the Rust struct):
     *   [RSP+0]  mboot_info: usize  (8 bytes, from RBX)
     *   [RSP+8]  mboot_magic: u32   (4 bytes, from EAX) + 4 bytes padding
     *
     * Note: KernelArguments is { mboot_magic: u32, mboot_info: usize }.
     * With Rust's default repr, u32 is at offset 0 and usize at offset 8
     * (due to alignment). We push mboot_info first (higher address), then
     * magic (lower address) so the struct is laid out correctly.
     */
    push %r13         /* mboot_info (usize) at RSP+8 */
    push %r12         /* mboot_magic (u32, zero-extended) at RSP+0 */

    /* RDI = pointer to KernelArguments on the stack (System V AMD64 ABI). */
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

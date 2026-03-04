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
 * Entry point for x86_64 kernel.
 *
 * The VMM (uservm) sets up long mode before starting the guest:
 *  - CR4.PAE is enabled
 *  - CR0.PG is enabled
 *  - EFER.LME and EFER.LMA are set
 *  - CR3 points to identity-mapped page tables
 *  - CS is a 64-bit code segment
 *
 * On entry:
 *  - RAX: Boot magic value (0x12345678)
 *  - RBX: Encoded initrd location + size
 *  - RIP: This entry point
 */
.align 8
.globl _do_start
_do_start:

    /*
     * RAX and RBX registers store boot information.
     * Save them before we trash them.
     */
    movq %rax, %r12          /* Save value of RAX.                 */
    movq %rbx, %r13          /* Save value of RBX.                 */

    /*
     * Fill BSS section with zeros.
     * We cannot do this later, because the stack is in the BSS.
     */
    movq $__BSS_START, %rdi  /* Start address of BSS section.      */
    movq $__BSS_END, %rcx    /* End address of BSS section.        */
    subq %rdi, %rcx          /* Calculate the size of BSS section. */
    xorq %rax, %rax          /* Zero out RAX.                      */
    cld                      /* Increment RDI register.            */
    rep stosb                /* Fill BSS section with zeros.       */

    /*
     * Fill the boot stack guard page with a watermark pattern.
     */
    movq $kstack_guard, %rdi         /* Start of guard page.       */
    movq $(PAGE_SIZE / 4), %rcx      /* Number of 32-bit words.    */
    movl $KSTACK_GUARD_PATTERN, %eax /* Watermark pattern.         */
    rep stosl                        /* Fill guard page.           */

    /* Reset stack. */
    movq $kstack, %rsp /* Stack pointer. */
    movq %rsp, %rbp    /* Frame pointer. */

    /*
     * Clear all general purpose registers,
     * so we have a deterministic startup.
     */
    xorq %rax, %rax
    xorq %rbx, %rbx
    xorq %rcx, %rcx
    xorq %rdx, %rdx
    xorq %rsi, %rsi
    xorq %rdi, %rdi
    xorq %r8, %r8
    xorq %r9, %r9
    xorq %r10, %r10
    xorq %r11, %r11

    /*
     * Call kernel main function.
     * x86_64 SysV ABI: first arg in RDI.
     * We pass a pointer to boot info on the stack.
     *
     * Push boot info onto the stack and pass RSP as first arg.
     */
    pushq %r13        /* RBX value (initrd info). */
    pushq %r12        /* RAX value (boot magic).  */
    movq %rsp, %rdi   /* First argument: pointer to boot info on stack. */

    call kmain
    addq $16, %rsp

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

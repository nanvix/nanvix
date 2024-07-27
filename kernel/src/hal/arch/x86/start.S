/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*============================================================================*
 * Constants                                                                  *
 *============================================================================*/

/* Page size. */
#define PAGE_SIZE 4096

/* Kernel stack size. */
#define KSTACK_SIZE (8 * PAGE_SIZE)

/* Multiboot2 magic constant */
#define MBOOT2_HEADER_MAGIC 0xe85250d6

/* Multiboot2 architecture ID. */
#define MBOOT2_ARCHITECTURE_I386 0

/* Multiboot2 header tag types. */
#define MBOOT2_HEADER_TAG_END 0

/*============================================================================*
 * Imported Symbols                                                           *
 *============================================================================*/

.extern kmain
.extern __BSS_START
.extern __BSS_END

/*============================================================================*
 * Exported Symbols                                                           *
 *============================================================================*/

.globl _do_start
.globl kstack

/*============================================================================*
 * Bootstrap Section                                                          *
 *============================================================================*/

.section .bootstrap,"ax",@progbits

/*----------------------------------------------------------------------------*
 * mboot_header                                                           *
 *----------------------------------------------------------------------------*/

/*
 *  Multiboot2 header.
 */
.align 8
mboot_header:
        .long MBOOT2_HEADER_MAGIC             /* Magic number. */
        .long MBOOT2_ARCHITECTURE_I386        /* Architecture  */
        .long mboot_header_end - mboot_header /* Header Length */
        /*  Checksum */
        .long -(MBOOT2_HEADER_MAGIC + MBOOT2_ARCHITECTURE_I386 + (mboot_header_end - mboot_header))
        framebuffer_tag_end:
        .short MBOOT2_HEADER_TAG_END
        .short 0
        .long 8
mboot_header_end:

/*----------------------------------------------------------------------------*
 * _do_start()                                                                *
 *----------------------------------------------------------------------------*/

/*
 * Entry point.
 */
.align 4
_do_start:

    /*
     * Fill BSS section with zeros.
     * We cannot do this later, becase the stack is in the BSS.
     * We preserve values of EAX and EBX, as they store boot information.
     */
    movl %eax, %edx          /* Save value of EAX.                 */
    movl $__BSS_START, %edi  /* Start address of BSS section.      */
    movl $__BSS_END, %ecx    /* End address of BSS section.        */
    subl %edi, %ecx          /* Calculate the size of BSS section. */
    xorl %eax, %eax          /* Zero out EAX.                      */
    cld                      /* Increment EDI register.            */
    rep stosb                /* Fill BSS section with zeros.       */
    movl %edx, %eax          /* Restore EAX register.              */

    /* Reset stack. */
    movl $kstack, %esp /* Stack pointer. */
    movl %esp, %ebp    /* Frame pointer. */

    /* Save multiboot parameters on the stack. */
    push %ebx /* Address of multiboot2 structure. */
    push %eax /* Magic value.                     */

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

    /* Cleanup multiboot parameters. */
    addl $8, %esp

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

.align  PAGE_SIZE
.space KSTACK_SIZE
kstack:

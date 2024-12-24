# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

/*================================================================================================*
 * Exported Symbols                                                                               *
 *================================================================================================*/

.global _start

/*================================================================================================*
 * Imported Symbols                                                                               *
 *================================================================================================*/

.extern kmain

/*================================================================================================*
 * Macros                                                                                         *
 *================================================================================================*/

/* Protection enable. */
#define CR0_PE 0x00000001

/* Null Segment. */
#define NULL_SEGMENT \
    .word 0, 0;     \
    .byte 0, 0, 0, 0

/* Generic kernel segment. */
#define KERNEL_SEGMENT(type,base,lim)                  \
    .word (((lim) >> 12) & 0xffff), ((base) & 0xffff); \
    .byte (((base) >> 16) & 0xff), (0x90 | (type)),    \
           (0xC0 | (((lim) >> 28) & 0xf)), (((base) >> 24) & 0xff)


/* Executable segment. */
#define SEG_X 0x8

/* Writable segment (non-executable segments). */
#define SEG_W 0x2

/* Readable segment (executable segments). */
#define SEG_R 0x2

/* Kernel code segment selector. */
#define KERNEL_CODE_SEGMENT_SELECTOR 1

/* Kernel data segment selector. */
#define KERNEL_DATA_SEGMENT_SELECTOR 2

/*================================================================================================*
 * Text Section                                                                                   *
 *================================================================================================*/

.section .text

/*------------------------------------------------------------------------------------------------*
 * _start()                                                                                       *
 *------------------------------------------------------------------------------------------------*/

.code16
.align 4
_start:

    /* Disable interrupts. */
    cli

    /* Clear segment registers */
    xorw %ax, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %fs
    movw %ax, %gs
    movw %ax, %ss

    /* Clear general purpose registers. */
    xorl %eax, %eax
    xorl %ebx, %ebx
    xorl %ecx, %ecx
    xorl %edx, %edx
    xorl %edi, %edi
    xorl %esi, %esi

    /* Switch to protected mode. */
    lgdt  gdtptr
    movl  %cr0, %eax
    orl   $CR0_PE, %eax
    movl  %eax, %cr0

    /* Load code segment register. */
    ljmp $(KERNEL_CODE_SEGMENT_SELECTOR<<3), $start32

/*------------------------------------------------------------------------------------------------*
 * start32()                                                                                      *
 *------------------------------------------------------------------------------------------------*/

.code32
.align 4
start32:
    /* Load data segment registers. */
    movw    $(KERNEL_DATA_SEGMENT_SELECTOR<<3), %ax
    movw    %ax, %ds
    movw    %ax, %es
    movw    %ax, %ss
    movw    %ax, %fs
    movw    %ax, %gs

    /* Set up the stack pointer. */
    movl $kstack, %esp
    movl $kstack, %ebp

    /* Call kmain. */
    call kmain

    /* Shutdown. */
    htlt:
        hlt
        jmp htlt


/*================================================================================================*
 * Data Section                                                                                   *
 *================================================================================================*/

.section .data

.align 4
gdt:
    NULL_SEGMENT
    KERNEL_SEGMENT(SEG_X|SEG_R, 0, 0xffffffff)
    KERNEL_SEGMENT(SEG_W, 0, 0xffffffff)

gdtptr:
    .word  (gdtptr - gdt - 1)
    .long  gdt

/*================================================================================================*
 * BSS Section                                                                                    *
 *================================================================================================*/

.section .bss

.align 4096
.space 4096
kstack:

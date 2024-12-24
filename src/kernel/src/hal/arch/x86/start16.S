/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*============================================================================*
 * Constants                                                                  *
 *============================================================================*/

#define SEG_NULLASM                                             \
        .word 0, 0;                                             \
        .byte 0, 0, 0, 0

# The 0xC0 means the limit is in 4096-byte units
# and (for executable segments) 32-bit mode.
#define SEG_ASM(type,base,lim)                                  \
        .word (((lim) >> 12) & 0xffff), ((base) & 0xffff);      \
        .byte (((base) >> 16) & 0xff), (0x90 | (type)),         \
                (0xC0 | (((lim) >> 28) & 0xf)), (((base) >> 24) & 0xff)

#define STA_X     0x8       // Executable segment
#define STA_W     0x2       // Writeable (non-executable segments)
#define STA_R     0x2       // Readable (executable segments)

/* Check if compiling for a PC platform */
#ifdef __pc__
/* Enable multiboot header. */
#define __multiboot__
#endif

/*============================================================================*
 * Imported Symbols                                                           *
 *============================================================================*/

.extern _do_ap_start

/*============================================================================*
 * Trampoline Section                                                         *
 *============================================================================*/

.section .trampoline,"ax",@progbits

/*----------------------------------------------------------------------------*
 * _ap_trampoline()                                                           *
 *----------------------------------------------------------------------------*/

/*
 * Entry point.
 */
#if !defined(__multiboot__)
.code16
.align 4
.globl _do_start
_do_start:

    # Zero data segment registers DS, ES, and SS.
    xorw  %dx,%dx
    movw  %dx,%ds
    movw  %dx,%es
    movw  %dx,%fs
    movw  %dx,%gs
    movw  %dx,%ss

    lgdt  gdtptr
    movl  %cr0, %edx
    orl   $1, %edx
    mov   %edx, %cr0

    .extern _do_start2
    jmpl $0x8, $_do_start2
#endif

.code16
.align 4
.globl _ap_trampoline
_ap_trampoline:
  cli

  # Zero data segment registers DS, ES, and SS.
  xorw    %ax,%ax
  movw    %ax,%ds
  movw    %ax,%es
  movw    %ax,%fs
  movw    %ax,%gs
  movw    %ax,%ss

  lgdt   gdtptr
  movl    %cr0, %eax
  orl     $1, %eax
  mov    %eax, %cr0
  jmpl $0x8, $_do_ap_start

.p2align 2
gdt:
  SEG_NULLASM
  SEG_ASM(STA_X|STA_R, 0, 0xffffffff)
  SEG_ASM(STA_W, 0, 0xffffffff)

gdtptr:
  .word   (gdtptr - gdt - 1)
  .long   gdt

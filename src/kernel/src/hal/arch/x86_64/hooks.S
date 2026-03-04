/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#define WORD_SIZE 8  /** Number of bytes in a 64-bit word. */

/**
 * @brief Hardware-saved execution context size (in bytes).
 *
 * In 64-bit mode, the CPU pushes: SS, RSP, RFLAGS, CS, RIP (and error code
 * for some exceptions). That is 5 × 8 = 40 bytes (without error code).
 */
#define CONTEXT_HW_SIZE 40

/**
 * @brief Software-saved execution context size (in bytes).
 *
 * We save: RAX, RBX, RCX, RDX, RBP, RSI, RDI, R8-R15 = 15 registers × 8
 * Plus: DS, ES, FS, GS (as 64-bit slots for alignment) = 4 × 8
 * Plus: CR3, RSP0 = 2 × 8
 * Total: (15 + 4 + 2) × 8 = 168
 */
#define CONTEXT_SW_SIZE 168

/**
 * @brief Execution context size (in bytes).
 */
#define CONTEXT_SIZE (CONTEXT_HW_SIZE + CONTEXT_SW_SIZE)

/* Offsets to the 64-bit TSS structure. */
#define TSS_RSP0 4    /* Ring 0 stack pointer (u64).   */

/**
 * @name Offsets to the 64-bit Context Structure (software-saved portion)
 */
#define CONTEXT_RSP0    0   /** Ring 0 Stack Pointer             */
#define CONTEXT_CR3     8   /** Page table base (PML4)           */
#define CONTEXT_GS      16  /** GS segment (stored as u64)       */
#define CONTEXT_FS      24  /** FS segment (stored as u64)       */
#define CONTEXT_ES      32  /** ES segment (stored as u64)       */
#define CONTEXT_DS      40  /** DS segment (stored as u64)       */
#define CONTEXT_R15     48  /** R15                              */
#define CONTEXT_R14     56  /** R14                              */
#define CONTEXT_R13     64  /** R13                              */
#define CONTEXT_R12     72  /** R12                              */
#define CONTEXT_R11     80  /** R11                              */
#define CONTEXT_R10     88  /** R10                              */
#define CONTEXT_R9      96  /** R9                               */
#define CONTEXT_R8      104 /** R8                               */
#define CONTEXT_RDI     112 /** RDI                              */
#define CONTEXT_RSI     120 /** RSI                              */
#define CONTEXT_RBP     128 /** RBP                              */
#define CONTEXT_RDX     136 /** RDX                              */
#define CONTEXT_RCX     144 /** RCX                              */
#define CONTEXT_RBX     152 /** RBX                              */
#define CONTEXT_RAX     160 /** RAX                              */
/* After software context: error code (8), RIP (8), CS (8), RFLAGS (8), RSP (8), SS (8) */
#define CONTEXT_ERR     168 /** Error code                       */
#define CONTEXT_RIP     176 /** RIP (pushed by hardware)         */
#define CONTEXT_CS      184 /** CS  (pushed by hardware)         */
#define CONTEXT_RFLAGS  192 /** RFLAGS (pushed by hardware)      */
#define CONTEXT_RSP     200 /** RSP (pushed by hardware)         */
#define CONTEXT_SS      208 /** SS  (pushed by hardware)         */

/**
 * @brief Exception information size (in bytes).
 */
#define EXCEPTION_SIZE 32

/**
 * @name Offsets to the Exception Information Structure
 */
#define EXCEPTION_NR    0   /** Exception Number     */
#define EXCEPTION_ERR   8   /** Error Code           */
#define EXCEPTION_DATA  16  /** Faulting Address     */
#define EXCEPTION_CODE  24  /** Faulting Instruction */

/*
 * Offset to exception structure.
 */
#define EXCEPTION_SKIP -CONTEXT_SW_SIZE - EXCEPTION_SIZE + EXCEPTION_ERR

/*============================================================================*
 * Macros                                                                     *
 *============================================================================*/

/*----------------------------------------------------------------------------*
 * context_save()                                                             *
 *----------------------------------------------------------------------------*/

.macro context_save ret

    sub $CONTEXT_SW_SIZE, %rsp

    /* Save general purpose registers. */
    mov %rax, CONTEXT_RAX(%rsp)
    mov %rbx, CONTEXT_RBX(%rsp)
    mov %rcx, CONTEXT_RCX(%rsp)
    mov %rdx, CONTEXT_RDX(%rsp)
    mov %rbp, CONTEXT_RBP(%rsp)
    mov %rsi, CONTEXT_RSI(%rsp)
    mov %rdi, CONTEXT_RDI(%rsp)
    mov %r8,  CONTEXT_R8(%rsp)
    mov %r9,  CONTEXT_R9(%rsp)
    mov %r10, CONTEXT_R10(%rsp)
    mov %r11, CONTEXT_R11(%rsp)
    mov %r12, CONTEXT_R12(%rsp)
    mov %r13, CONTEXT_R13(%rsp)
    mov %r14, CONTEXT_R14(%rsp)
    mov %r15, CONTEXT_R15(%rsp)

    /* Save segment registers (as 64-bit values for alignment). */
    xor %rax, %rax
    mov %ds, %ax
    mov %rax, CONTEXT_DS(%rsp)
    mov %es, %ax
    mov %rax, CONTEXT_ES(%rsp)
    mov %fs, %ax
    mov %rax, CONTEXT_FS(%rsp)
    mov %gs, %ax
    mov %rax, CONTEXT_GS(%rsp)

    mov %rsp, \ret

.endm

/*----------------------------------------------------------------------------*
 * context_restore()                                                          *
 *----------------------------------------------------------------------------*/

.macro context_restore

    /* Restore general purpose registers. */
    mov CONTEXT_RAX(%rsp), %rax
    mov CONTEXT_RBX(%rsp), %rbx
    mov CONTEXT_RCX(%rsp), %rcx
    mov CONTEXT_RDX(%rsp), %rdx
    mov CONTEXT_RBP(%rsp), %rbp
    mov CONTEXT_RSI(%rsp), %rsi
    mov CONTEXT_RDI(%rsp), %rdi
    mov CONTEXT_R8(%rsp),  %r8
    mov CONTEXT_R9(%rsp),  %r9
    mov CONTEXT_R10(%rsp), %r10
    mov CONTEXT_R11(%rsp), %r11
    mov CONTEXT_R12(%rsp), %r12
    mov CONTEXT_R13(%rsp), %r13
    mov CONTEXT_R14(%rsp), %r14
    mov CONTEXT_R15(%rsp), %r15

    /* Restore segment registers. */
    push CONTEXT_DS(%rsp)
    pop %rax
    mov %ax, %ds
    mov %ax, %es
    push CONTEXT_FS(%rsp)
    pop %rax
    mov %ax, %fs
    push CONTEXT_GS(%rsp)
    pop %rax
    mov %ax, %gs

    add $CONTEXT_SW_SIZE, %rsp

.endm

/*----------------------------------------------------------------------------*
 * _do_excp()                                                                 *
 *----------------------------------------------------------------------------*/

.macro _do_excp_noerr_code, number
    _do_excp\()\number:
        pushq $0            /* Fake error code.          */
        context_save %rdi
        mov $(\number), %rsi
        xor %rdx, %rdx
        jmp _do_excp
.endm

.macro _do_excp_err_code, number
    _do_excp\()\number:
        context_save %rdi
        mov $(\number), %rsi
        xor %rdx, %rdx
        jmp _do_excp
.endm

.macro _do_excp_err2_code, number
    _do_excp\()\number:
        context_save %rdi
        mov $(\number), %rsi
        mov %cr2, %rdx
        jmp _do_excp
.endm

/*----------------------------------------------------------------------------*
 * _do_hwint()                                                                *
 *----------------------------------------------------------------------------*/

.macro _do_hwint num
    _do_hwint\()\num:
        pushq $0            /* Fake error code for uniform stack frame. */
        context_save %rdi   /* First arg: context pointer. */
        mov $(\num), %rdi   /* First arg: interrupt number. */
        call do_interrupt
        context_restore
        add $WORD_SIZE, %rsp /* Pop fake error code. */
        iretq
.endm

/*============================================================================*
 * Text Section                                                               *
 *============================================================================*/

.section .text,"ax",@progbits

/*----------------------------------------------------------------------------*
 * Exported Symbols                                                           *
 *----------------------------------------------------------------------------*/

.extern do_exception
.extern do_kcall
.extern do_interrupt

/* Exception hooks. */
.globl _do_excp0
.globl _do_excp1
.globl _do_excp2
.globl _do_excp3
.globl _do_excp4
.globl _do_excp5
.globl _do_excp6
.globl _do_excp7
.globl _do_excp8
.globl _do_excp9
.globl _do_excp10
.globl _do_excp11
.globl _do_excp12
.globl _do_excp13
.globl _do_excp14
.globl _do_excp15
.globl _do_excp16
.globl _do_excp17
.globl _do_excp18
.globl _do_excp19
.globl _do_excp20
.globl _do_excp30

/* Kernel call hook. */
.globl _do_kcall

/* Hardware interrupt hooks. */
.globl _do_hwint0
.globl _do_hwint1
.globl _do_hwint2
.globl _do_hwint3
.globl _do_hwint4
.globl _do_hwint5
.globl _do_hwint6
.globl _do_hwint7
.globl _do_hwint8
.globl _do_hwint9
.globl _do_hwint10
.globl _do_hwint11
.globl _do_hwint12
.globl _do_hwint13
.globl _do_hwint14
.globl _do_hwint15

/* Other */
.global __context_switch
.global __leave_kernel
.global __leave_kernel_to_user_mode
.global __phys_memcpy
.global __phys_memset

/*----------------------------------------------------------------------------*
 * _do_excp()                                                                 *
 *----------------------------------------------------------------------------*/

/* Exception hooks. */
_do_excp_noerr_code  0 /* Division-by-Zero Error      */
_do_excp_noerr_code  1 /* Debug Exception             */
_do_excp_noerr_code  2 /* Non-Maskable Interrupt      */
_do_excp_noerr_code  3 /* Breakpoint Exception        */
_do_excp_noerr_code  4 /* Overflow Exception          */
_do_excp_noerr_code  5 /* Bounds Check Exception      */
_do_excp_noerr_code  6 /* Invalid Opcode Exception    */
_do_excp_noerr_code  7 /* Coprocessor Not Available   */
_do_excp_err_code    8 /* Double Fault                */
_do_excp_noerr_code  9 /* Coprocessor Segment Overrun */
_do_excp_err_code   10 /* Invalid TSS                 */
_do_excp_err_code   11 /* Segment Not Present         */
_do_excp_err_code   12 /* Stack Segment Fault         */
_do_excp_err_code   13 /* General Protection Fault    */
_do_excp_err2_code  14 /* Page Fault                  */
_do_excp_noerr_code 15 /* Reserved                    */
_do_excp_noerr_code 16 /* Floating Point Exception    */
_do_excp_err_code   17 /* Alignment Check Exception   */
_do_excp_noerr_code 18 /* Machine Check Exception     */
_do_excp_noerr_code 19 /* SMID Unit Exception         */
_do_excp_noerr_code 20 /* Virtual Exception           */
_do_excp_err_code   30 /* Security Exception.         */

/*
 * Low-level exception handler dispatcher.
 *
 * On entry (System V AMD64 ABI):
 *   RDI = pointer to saved context
 *   RSI = exception number
 *   RDX = auxiliary data (e.g., CR2 for page fault)
 */
_do_excp:
    /* Build exception information on stack. */
    mov CONTEXT_RIP(%rdi), %rcx
    sub $EXCEPTION_SIZE, %rsp
    mov %rsi, EXCEPTION_NR(%rsp)
    mov %rdx, EXCEPTION_DATA(%rsp)
    mov %rcx, EXCEPTION_CODE(%rsp)

    /* Call high-level exception dispatcher.
     * RDI = context pointer (already set)
     * RSI = pointer to exception info */
    mov %rsp, %rsi
    call do_exception
    add $EXCEPTION_SIZE, %rsp

    context_restore

    /* Pop error code. */
    add $WORD_SIZE, %rsp

    jmp __leave_kernel

/*----------------------------------------------------------------------------*
 * _do_kcall()                                                                *
 *----------------------------------------------------------------------------*/

/*
 * Kernel call hook (64-bit).
 *
 * System V AMD64 ABI: arguments in RDI, RSI, RDX, RCX, R8, R9.
 * We pass syscall arguments directly to do_kcall.
 */
_do_kcall:
    /* Save callee-saved registers. */
    push %rbp
    push %rbx
    push %r12
    push %r13
    push %r14
    push %r15

    mov %rsp, %rbp

    /* Call kernel call handler.
     * Arguments are already in the correct registers per System V ABI:
     *   RDI = arg0 (syscall number)
     *   RSI = arg1
     *   RDX = arg2
     *   RCX = arg3
     *   R8  = arg4
     */
    call do_kcall

    /* Restore callee-saved registers. */
    pop %r15
    pop %r14
    pop %r13
    pop %r12
    pop %rbx
    pop %rbp

    jmp __leave_kernel

/*----------------------------------------------------------------------------*
 * _do_hwint()                                                                *
 *----------------------------------------------------------------------------*/

/* Hardware interrupt hooks. */
_do_hwint  0
_do_hwint  1
_do_hwint  2
_do_hwint  3
_do_hwint  4
_do_hwint  5
_do_hwint  6
_do_hwint  7
_do_hwint  8
_do_hwint  9
_do_hwint 10
_do_hwint 11
_do_hwint 12
_do_hwint 13
_do_hwint 14
_do_hwint 15

/*----------------------------------------------------------------------------*
 * __context_switch()                                                         *
 *----------------------------------------------------------------------------*/

/*
 * Context switch (64-bit).
 *
 * void __context_switch(
 *     ContextInformation *from,   // RDI
 *     ContextInformation *to,     // RSI
 *     Tss *tss                    // RDX
 * );
 */
__context_switch:

    /* Save callee-saved registers to 'from' context. */
    mov %rbx, CONTEXT_RBX(%rdi)
    mov %rbp, CONTEXT_RBP(%rdi)
    mov %rsi, CONTEXT_RSI(%rdi)
    mov %r12, CONTEXT_R12(%rdi)
    mov %r13, CONTEXT_R13(%rdi)
    mov %r14, CONTEXT_R14(%rdi)
    mov %r15, CONTEXT_R15(%rdi)
    mov %rsp, CONTEXT_RSP(%rdi)
    pushfq
    pop CONTEXT_RFLAGS(%rdi)

    /* Save address space. */
    mov %cr3, %rax
    mov %rax, CONTEXT_CR3(%rdi)

    /* Restore callee-saved registers from 'to' context. */
    mov CONTEXT_RBX(%rsi), %rbx
    mov CONTEXT_RBP(%rsi), %rbp
    mov CONTEXT_R12(%rsi), %r12
    mov CONTEXT_R13(%rsi), %r13
    mov CONTEXT_R14(%rsi), %r14
    mov CONTEXT_R15(%rsi), %r15
    mov CONTEXT_RSP(%rsi), %rsp
    push CONTEXT_RFLAGS(%rsi)
    popfq

    /* Restore address space. */
    mov CONTEXT_CR3(%rsi), %rax
    mov %rax, %cr3

    /* Update RSP0 in TSS for privilege transitions. */
    mov CONTEXT_RSP0(%rsi), %rax
    mov %rax, TSS_RSP0(%rdx)

    ret

/*----------------------------------------------------------------------------*
 * __leave_kernel()                                                           *
 *----------------------------------------------------------------------------*/

__leave_kernel_to_user_mode:
    /* TODO: Implement user mode transition for 64-bit. */

__leave_kernel:
    iretq

/*----------------------------------------------------------------------------*
 * __phys_memcpy()                                                            *
 *----------------------------------------------------------------------------*/

/*
 * Physical memory copy (64-bit).
 *
 * void __phys_memcpy(void *dest, const void *src, size_t size);
 *   RDI = dest, RSI = src, RDX = size
 *
 * In 64-bit mode with identity mapping, we can copy directly.
 */
__phys_memcpy:
    mov %rdx, %rcx
    test %rcx, %rcx
    jz 1f
    cld
    rep movsb
1:
    ret

/*----------------------------------------------------------------------------*
 * __phys_memset()                                                            *
 *----------------------------------------------------------------------------*/

/*
 * Physical memory set (64-bit).
 *
 * void __phys_memset(void *dest, u8 value, size_t size);
 *   RDI = dest, RSI = value, RDX = size
 */
__phys_memset:
    mov %rdx, %rcx
    mov %rsi, %rax
    cld
    rep stosb
    ret

/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#define QWORD_SIZE 8 /** Number of bytes in a quad-word. */

/**
 * @brief Hardware-saved execution context size (in bytes).
 * On x86_64, the CPU pushes SS, RSP, RFLAGS, CS, RIP (and optionally error code).
 */
#define CONTEXT_HW_SIZE 40

/**
 * @brief Software-saved execution context size (in bytes).
 * We save: rsp0, cr3, r15-r8, rbp, rsi, rdi, rdx, rcx, rbx, rax = 17 * 8 = 136
 */
#define CONTEXT_SW_SIZE 136

/**
 * @brief Execution context size (in bytes).
 */
#define CONTEXT_SIZE (CONTEXT_HW_SIZE + CONTEXT_SW_SIZE)

/**
 * @name Offsets to the Context Structure
 */
#define CONTEXT_RSP0    0    /* Ring 0 Stack Pointer                  */
#define CONTEXT_CR3     8    /* Page Directory Register               */
#define CONTEXT_R15     16   /* R15 Register                          */
#define CONTEXT_R14     24   /* R14 Register                          */
#define CONTEXT_R13     32   /* R13 Register                          */
#define CONTEXT_R12     40   /* R12 Register                          */
#define CONTEXT_R11     48   /* R11 Register                          */
#define CONTEXT_R10     56   /* R10 Register                          */
#define CONTEXT_R9      64   /* R9 Register                           */
#define CONTEXT_R8      72   /* R8 Register                           */
#define CONTEXT_RBP     80   /* RBP Register                          */
#define CONTEXT_RSI     88   /* RSI Register                          */
#define CONTEXT_RDI     96   /* RDI Register                          */
#define CONTEXT_RDX     104  /* RDX Register                          */
#define CONTEXT_RCX     112  /* RCX Register                          */
#define CONTEXT_RBX     120  /* RBX Register                          */
#define CONTEXT_RAX     128  /* RAX Register                          */
#define CONTEXT_ERR     136  /* Error code                            */
#define CONTEXT_RIP     144  /* RIP Register                          */
#define CONTEXT_CS      152  /* Code Segment Register                 */
#define CONTEXT_RFLAGS  160  /* RFLAGS Register                       */
#define CONTEXT_RSP     168  /* RSP Register                          */
#define CONTEXT_SS      176  /* Stack Segment Register                */

/* Offsets to the x86_64 TSS structure. */
#define TSS_RSP0 4    /* Ring 0 stack pointer (low 32 bits).   */

/**
 * @brief Exception information size (in bytes).
 */
#define EXCEPTION_SIZE 32

/**
 * @name Offsets to the Exception Information Structure
 */
#define EXCEPTION_NR    0    /* Exception Number     */
#define EXCEPTION_ERR   8    /* Error Code           */
#define EXCEPTION_DATA  16   /* Faulting Address     */
#define EXCEPTION_CODE  24   /* Faulting Instruction */

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

    subq $CONTEXT_SW_SIZE, %rsp

    /* Save general purpose registers. */
    movq %rax, CONTEXT_RAX(%rsp)
    movq %rbx, CONTEXT_RBX(%rsp)
    movq %rcx, CONTEXT_RCX(%rsp)
    movq %rdx, CONTEXT_RDX(%rsp)
    movq %rdi, CONTEXT_RDI(%rsp)
    movq %rsi, CONTEXT_RSI(%rsp)
    movq %rbp, CONTEXT_RBP(%rsp)
    movq %r8,  CONTEXT_R8(%rsp)
    movq %r9,  CONTEXT_R9(%rsp)
    movq %r10, CONTEXT_R10(%rsp)
    movq %r11, CONTEXT_R11(%rsp)
    movq %r12, CONTEXT_R12(%rsp)
    movq %r13, CONTEXT_R13(%rsp)
    movq %r14, CONTEXT_R14(%rsp)
    movq %r15, CONTEXT_R15(%rsp)

    movq %rsp, \ret

.endm

/*----------------------------------------------------------------------------*
 * context_restore()                                                          *
 *----------------------------------------------------------------------------*/

.macro context_restore

    /* Restore general purpose registers. */
    movq CONTEXT_RAX(%rsp), %rax
    movq CONTEXT_RBX(%rsp), %rbx
    movq CONTEXT_RCX(%rsp), %rcx
    movq CONTEXT_RDX(%rsp), %rdx
    movq CONTEXT_RDI(%rsp), %rdi
    movq CONTEXT_RSI(%rsp), %rsi
    movq CONTEXT_RBP(%rsp), %rbp
    movq CONTEXT_R8(%rsp),  %r8
    movq CONTEXT_R9(%rsp),  %r9
    movq CONTEXT_R10(%rsp), %r10
    movq CONTEXT_R11(%rsp), %r11
    movq CONTEXT_R12(%rsp), %r12
    movq CONTEXT_R13(%rsp), %r13
    movq CONTEXT_R14(%rsp), %r14
    movq CONTEXT_R15(%rsp), %r15

    addq $CONTEXT_SW_SIZE, %rsp

.endm

/*----------------------------------------------------------------------------*
 * _do_excp()                                                                 *
 *----------------------------------------------------------------------------*/

.macro _do_excp_noerr_code, number
    _do_excp\()\number:
        pushq $0
        xchg %rax, (%rsp)
        xchg %rax, EXCEPTION_SKIP(%rsp)
        xchg %rax, (%rsp)
        context_save %rdi
        movq $(\number), %rsi
        movq $0, %rdx
        jmp _do_excp
.endm

.macro _do_excp_err_code, number
    _do_excp\()\number:
        xchg %rax, (%rsp)
        xchg %rax, EXCEPTION_SKIP(%rsp)
        xchg %rax, (%rsp)
        context_save %rdi
        movq $(\number), %rsi
        movq $0, %rdx
        jmp _do_excp
.endm

.macro _do_excp_err2_code, number
    _do_excp\()\number:
        xchg %rax, (%rsp)
        xchg %rax, EXCEPTION_SKIP(%rsp)
        xchg %rax, (%rsp)
        context_save %rdi
        movq $(\number), %rsi
        movq %cr2, %rdx
        jmp _do_excp
.endm

/*----------------------------------------------------------------------------*
 * _do_hwint()                                                                *
 *----------------------------------------------------------------------------*/

.macro _do_hwint num
    _do_hwint\()\num:
        context_save %rax
        movq $(\num), %rdi
        call do_interrupt
        context_restore
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
.global __phys_memcpy32
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
 * On entry (x86_64 SysV ABI):
 *  - RDI: pointer to saved context
 *  - RSI: exception number
 *  - RDX: faulting address (CR2 for page faults, 0 otherwise)
 */
_do_excp:

    /* Build exception information on the stack. */
    movq CONTEXT_RIP(%rdi), %rcx
    subq $EXCEPTION_SIZE, %rsp
    movq %rsi, EXCEPTION_NR(%rsp)
    movq %rdx, EXCEPTION_DATA(%rsp)
    movq %rcx, EXCEPTION_CODE(%rsp)
    movq EXCEPTION_ERR(%rsp), %rcx
    movq %rcx, CONTEXT_ERR(%rdi)

    /* Call high-level exception dispatcher. */
    /* RDI already has context pointer. */
    movq %rsp, %rsi   /* Exception info pointer. */
    call do_exception
    addq $EXCEPTION_SIZE, %rsp

    context_restore

    /* Pop error code. */
    addq $QWORD_SIZE, %rsp

    jmp __leave_kernel

/*----------------------------------------------------------------------------*
 * _do_kcall()                                                                *
 *----------------------------------------------------------------------------*/

/*
 * Kernel call hook.
 *
 * On entry (from int 0x80):
 *  - RAX: kernel call number
 *  - RDI: arg0
 *  - RSI: arg1
 *  - RDX: arg2
 *  - R10: arg3
 */
_do_kcall:
    /*
     * Save callee-saved registers.
     */
    pushq %rbp
    pushq %r12
    pushq %r13
    pushq %r14
    pushq %r15
    pushq %rbx

    movq %rsp, %rbp

    /* Set up x86_64 SysV ABI arguments for do_kcall(kcall_nr, arg0, arg1, arg2, arg3). */
    /* RDI already holds arg0 but we need it for kcall_nr. Save args first. */
    movq %r10, %r8   /* arg3 → r8 (5th SysV param) */
    movq %rdx, %rcx  /* arg2 → rcx (4th SysV param) */
    movq %rsi, %rdx  /* arg1 → rdx (3rd SysV param) */
    movq %rdi, %rsi  /* arg0 → rsi (2nd SysV param) */
    movq %rax, %rdi  /* kcall_nr → rdi (1st SysV param) */

    /* Handle kernel call. */
    call do_kcall

    /*
     * Restore callee-saved registers.
     * Return value is in RAX.
     */
    popq %rbx
    popq %r15
    popq %r14
    popq %r13
    popq %r12
    popq %rbp

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
 * Context switch (x86_64).
 *
 * void __context_switch(
 *   ContextInformation *from,  // RDI
 *   ContextInformation *to,    // RSI
 *   Tss *tss                   // RDX
 * );
 */
__context_switch:

    /* Save execution context. */
    movq %rbx, CONTEXT_RBX(%rdi)
    movq %r12, CONTEXT_R12(%rdi)
    movq %r13, CONTEXT_R13(%rdi)
    movq %r14, CONTEXT_R14(%rdi)
    movq %r15, CONTEXT_R15(%rdi)
    movq %rbp, CONTEXT_RBP(%rdi)
    movq %rsp, CONTEXT_RSP(%rdi)
    pushfq
    popq CONTEXT_RFLAGS(%rdi)

    /* Save address space. */
    movq %cr3, %rax
    movq %rax, CONTEXT_CR3(%rdi)

    /* Restore execution context. */
    movq CONTEXT_RBX(%rsi), %rbx
    movq CONTEXT_R12(%rsi), %r12
    movq CONTEXT_R13(%rsi), %r13
    movq CONTEXT_R14(%rsi), %r14
    movq CONTEXT_R15(%rsi), %r15
    movq CONTEXT_RBP(%rsi), %rbp
    movq CONTEXT_RSP(%rsi), %rsp
    pushq CONTEXT_RFLAGS(%rsi)
    popfq

    /* Restore address space. */
    /* Only reload CR3 if the target context has a non-zero CR3. A zero CR3
       indicates the initial boot context before per-process page tables exist. */
    movq CONTEXT_CR3(%rsi), %rax
    testq %rax, %rax
    jz .Lskip_cr3
    movq %rax, %cr3
.Lskip_cr3:

    /* Update RSP0 on TSS. */
    movq CONTEXT_RSP0(%rsi), %rax
    movl %eax, TSS_RSP0(%rdx)
    shrq $32, %rax
    movl %eax, (TSS_RSP0 + 4)(%rdx)

    ret

/*----------------------------------------------------------------------------*
 * __leave_kernel()                                                           *
 *----------------------------------------------------------------------------*/

__leave_kernel_to_user_mode:

    /* Pop user function arguments pushed by forge_user_stack(). */
    /* Stack order: arg1 (top), arg0 — map to SysV ABI registers. */
    popq %rsi                  /* arg1 (second argument: envp). */
    popq %rdi                  /* arg0 (first argument: argp). */

    /* Clear volatile registers to avoid leaking kernel state. */
    /* Preserve RDI and RSI which carry user function arguments. */
    xorq %rax, %rax
    xorq %rbx, %rbx
    xorq %rcx, %rcx
    xorq %rdx, %rdx
    xorq %r8,  %r8
    xorq %r9,  %r9
    xorq %r10, %r10
    xorq %r11, %r11
    xorq %r12, %r12
    xorq %r13, %r13
    xorq %r14, %r14
    xorq %r15, %r15
    xorq %rbp, %rbp

    /* RSP now points at the iretq frame: RIP, CS, RFLAGS, RSP, SS. */

__leave_kernel:
    iretq

/*----------------------------------------------------------------------------*
 * __phys_memcpy()                                                            *
 *----------------------------------------------------------------------------*/

/*
 * Physical memory copy (x86_64).
 * Uses identity mapping - no need to disable paging.
 *
 * void __phys_memcpy(void *dest, const void *src, size_t size);
 * RDI = dest, RSI = src, RDX = size
 */
__phys_memcpy:
    movq %rdx, %rcx
    test %rcx, %rcx
    jz __phys_memcpy.done
    cld
    rep movsb
__phys_memcpy.done:
    ret

/*----------------------------------------------------------------------------*
 * __phys_memcpy32()                                                          *
 *----------------------------------------------------------------------------*/

__phys_memcpy32:
    movq %rdx, %rcx
    shrq $2, %rcx
    test %rcx, %rcx
    jz __phys_memcpy32.done
    cld
    rep movsl
__phys_memcpy32.done:
    ret

/*----------------------------------------------------------------------------*
 * __phys_memset()                                                            *
 *----------------------------------------------------------------------------*/

/*
 * void __phys_memset(void *dest, int value, size_t size);
 * RDI = dest, RSI = value, RDX = size
 */
__phys_memset:
    movq %rsi, %rax  /* value */
    movq %rdx, %rcx  /* count */
    cld
    rep stosb
    ret

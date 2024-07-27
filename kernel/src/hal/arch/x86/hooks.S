/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#define BYTE_SIZE 1  /** Number of bytes in a byte.        */
#define HWORD_SIZE 2 /** Number of bytes in a half-word.   */
#define WORD_SIZE 4  /** Number of bytes in a word.        */
#define DWORD_SIZE 8 /** Number of bytes in a double-word. */

/**
 * @brief Hardware-saved execution context size (in bytes).
 */
#define CONTEXT_HW_SIZE 24

/**
 * @brief Software-saved execution context size (in bytes).
 */
#define CONTEXT_SW_SIZE 52

/**
 * @brief Execution context size (in bytes).
 */
#define CONTEXT_SIZE (CONTEXT_HW_SIZE + CONTEXT_SW_SIZE)

/* Offsets to the tss structure. */
#define TSS_ESP0 4    /* Ring 0 stack pointer.         */
#define TSS_SS0 8     /* Ring 0 stack segment.         */
#define TSS_ESP1 12   /* Ring 1 stack pointer.         */
#define TSS_SS1 16    /* Ring 1 stack segment.         */
#define TSS_ESP2 20   /* Ring 2 stack pointer.         */
#define TSS_SS2 24    /* Ring 2 stack segment.         */
#define TSS_CR3 28    /* cr3.                          */
#define TSS_EIP 32    /* eip.                          */
#define TSS_EFLAGS 36 /* eflags.                       */
#define TSS_EAX 40    /* eax.                          */
#define TSS_ECX 44    /* ecx.                          */
#define TSS_EDX 48    /* edx.                          */
#define TSS_EBX 52    /* ebx.                          */
#define TSS_ESP 56    /* esp.                          */
#define TSS_EBP 60    /* ebp.                          */
#define TSS_ESI 64    /* esi.                          */
#define TSS_EDI 68    /* edi.                          */
#define TSS_ES 72     /* es.                           */
#define TSS_CS 76     /* cs.                           */
#define TSS_SS 80     /* ss.                           */
#define TSS_DS 84     /* ds.                           */
#define TSS_FS 88     /* fs.                           */
#define TSS_GS 92     /* gs.                           */
#define TSS_LDTR 96   /* LDT selector.                 */
#define TSS_IOMAP 100 /* IO map.                       */


/**
 * @name Offsets to the Context Structure
 */
/**@{*/
#define CONTEXT_ESP0 0    /** Ring 0 Stack Pointer Register         */
#define CONTEXT_CR3 4     /** Page Directory Register               */
#define CONTEXT_GS 8      /** Extra Data Segment #3 Register        */
#define CONTEXT_FS 12     /** Extra Data Segment #2 Register        */
#define CONTEXT_ES 16     /** Extra Data Segment #1 Register        */
#define CONTEXT_DS 20     /** Data Segment Register                 */
#define CONTEXT_EDI 24    /** Extended Destination Index Register   */
#define CONTEXT_ESI 28    /** Extended Source Index Register        */
#define CONTEXT_EBP 32    /** Extended Stack base Pointer Register  */
#define CONTEXT_EDX 36    /** Extended Accumulator #2 Register      */
#define CONTEXT_ECX 40    /** Extended Counter Register             */
#define CONTEXT_EBX 44    /** Extended Base Index Register          */
#define CONTEXT_EAX 48    /** Extended Accumulator #1 Register      */
#define CONTEXT_ERR 52    /* Error code                             */
#define CONTEXT_EIP 56    /** Extended Instruction Pointer Register */
#define CONTEXT_CS 60     /** Code Segment Register                 */
#define CONTEXT_EFLAGS 64 /** Extended Flags Register               */
#define CONTEXT_ESP 68    /** Extended Stack Pointer Register       */
#define CONTEXT_SS 72     /** Stack Segment Register                */
/**@}*/

/**
 * @brief Exception information size (in bytes).
 */
#define EXCEPTION_SIZE 16

/**
 * @name Offsets to the Exception Information Structure
 *
 * @see exception
 */
/**@{*/
#define EXCEPTION_NR 0    /** Offset to Exception Number     */
#define EXCEPTION_ERR 4   /** Offset to Error COde           */
#define EXCEPTION_DATA 8  /** Offset to Faulting Address     */
#define EXCEPTION_CODE 12 /** Offset to Faulting Instruction */
/**@}*/

/*
 * Offset to exception structure.
 */
#define EXCEPTION_SKIP -CONTEXT_SW_SIZE - EXCEPTION_SIZE + EXCEPTION_ERR


#define EFLAGS_ID (1 << 21) /* Able to use CPUID instruction? */

/*============================================================================*
 * Macros                                                                     *
 *============================================================================*/

/*----------------------------------------------------------------------------*
 * context_save()                                                             *
 *----------------------------------------------------------------------------*/

/*
 * The context_save() macro saves the content of general purpose
 * and segment registers in the stack of the caller. A pointer to the
 * saved execution context is saved into ret.
 *
 * Note that the extended instruction pointer register (eip), code
 * segment register (cs), extended flags register (eflags), extended
 * stack pointer register (esp) and stack segment register (ss) are
 * not saved by this macro, because they are automatically saved by
 * the hardware upon an exception/interrupt.
 */
.macro context_save, ret

    subl $CONTEXT_SW_SIZE, %esp

    /* Save general purpose registers. */
    movl %eax, CONTEXT_EAX(%esp)
    movl %ebx, CONTEXT_EBX(%esp)
    movl %ecx, CONTEXT_ECX(%esp)
    movl %edx, CONTEXT_EDX(%esp)
    movl %ebp, CONTEXT_EBP(%esp)
    movl %esi, CONTEXT_ESI(%esp)
    movl %edi, CONTEXT_EDI(%esp)

    /* Save segment registers. */
    movw %ds, CONTEXT_DS(%esp)
    movw %es, CONTEXT_ES(%esp)
    movw %fs, CONTEXT_FS(%esp)
    movw %gs, CONTEXT_GS(%esp)

    movl %esp, \ret

.endm

/*----------------------------------------------------------------------------*
 * context_restore()                                                          *
 *----------------------------------------------------------------------------*/

/*
 * The context_restore() macro restores the content of general
 * purpose and segment registers from the stack of the caller.
 *
 * It is up to the caller to switch the current stack to the target one.
 */
.macro context_restore

    /* Restore general purpose registers. */
    movl CONTEXT_EAX(%esp), %eax
    movl CONTEXT_EBX(%esp), %ebx
    movl CONTEXT_ECX(%esp), %ecx
    movl CONTEXT_EDX(%esp), %edx
    movl CONTEXT_EBP(%esp), %ebp
    movl CONTEXT_ESI(%esp), %esi
    movl CONTEXT_EDI(%esp), %edi

    /* Restore segment registers. */
    movw CONTEXT_DS(%esp), %ds
    movw CONTEXT_ES(%esp), %es
    movw CONTEXT_FS(%esp), %fs
    movw CONTEXT_GS(%esp), %gs

    addl $CONTEXT_SW_SIZE, %esp

    /*
     * Special registers are
     * restored by a further call to iret.
     */

.endm

/*----------------------------------------------------------------------------*
 * _do_excp()                                                                 *
 *----------------------------------------------------------------------------*/

/*
 * Exception hook.
 */
.macro _do_excp_noerr_code, number
	_do_excp\()\number:
        push $0
		xchg %eax, (%esp)
		xchg %eax, EXCEPTION_SKIP(%esp)
		xchg %eax, (%esp)
		context_save %eax
		movl $(\number), %ebx
		movl $0, %ecx
		jmp _do_excp
.endm

/*
 * Exception with error code.
 */
.macro _do_excp_err_code, number
	_do_excp\()\number:
		xchg %eax, (%esp)
		xchg %eax, EXCEPTION_SKIP(%esp)
		xchg %eax, (%esp)
		context_save %eax
		movl $(\number), %ebx
		movl $0, %ecx
		jmp _do_excp
.endm

/*
 * Exception with error code.
 */
.macro _do_excp_err2_code, number
	_do_excp\()\number:
		xchg %eax, (%esp)
		xchg %eax, EXCEPTION_SKIP(%esp)
		xchg %eax, (%esp)
		context_save %eax
		movl $(\number), %ebx
		movl %cr2, %ecx
		jmp _do_excp
.endm

/*----------------------------------------------------------------------------*
 * _do_hwint()                                                                *
 *----------------------------------------------------------------------------*/

/*
 * Low-level hardware interrupt dispatcher.
 */
.macro _do_hwint num
	_do_hwint\()\num:
		context_save %eax
		pushl $(\num)
		call do_interrupt
		addl $WORD_SIZE, %esp
		context_restore
		iret
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
.extern TSS

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
.global __physcopy
.global __has_cpuid

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
 */
_do_excp:
	/* Save exception information. */
	movl CONTEXT_EIP(%eax), %edx
	subl $EXCEPTION_SIZE, %esp
	movl %ebx, EXCEPTION_NR(%esp)
	movl %ecx, EXCEPTION_DATA(%esp)
	movl %edx, EXCEPTION_CODE(%esp)
	movl EXCEPTION_ERR(%esp), %ebx
	movl %ebx, CONTEXT_ERR(%eax)
	movl %esp, %ebx

	/* Call high-level exception dispatcher. */
	pushl %eax /* Execution context. */
	pushl %ebx /* Exception context. */
    call do_exception
	addl  $(2*WORD_SIZE), %esp
	addl $EXCEPTION_SIZE, %esp

	context_restore

	/* Pop error code. */
	addl $WORD_SIZE, %esp

    jmp __leave_kernel

/*----------------------------------------------------------------------------*
 * _do_kcall()                                                                *
 *----------------------------------------------------------------------------*/

/*
 * Kernel call hook.
 *
 * NOTE: This was registered as an interrupt gate in the IDT, thus we do not
 * clear/set the IF flag here, because that is taken care by the hardware.
 */
_do_kcall:

    /* Push kernel call parameters. */
    pushl %edi
    pushl %edx
    pushl %ecx
    pushl %ebx
    pushl %eax

    /* Handle kernel call. */
    call do_kcall

    /* Wipe out kernel call parameters. */
    addl $5*WORD_SIZE, %esp

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
 * Saves the execution context of the calling process.
 */
__context_switch:
    movl 4(%esp), %eax /* from */
    movl 8(%esp), %edx /* to */

    /* Save execution context. */
    movl %ebx, CONTEXT_EBX(%eax)
    movl %esi, CONTEXT_ESI(%eax)
    movl %edi, CONTEXT_EDI(%eax)
    movl %ebp, CONTEXT_EBP(%eax)
    movl %esp, CONTEXT_ESP(%eax)
    pushf
    pop CONTEXT_EFLAGS(%eax)

    /* Save address space. */
    movl %cr3, %ebx
    movl %ebx, CONTEXT_CR3(%eax)

    /* Restore execution context. */
    movl CONTEXT_EBX(%edx), %ebx
    movl CONTEXT_ESI(%edx), %esi
    movl CONTEXT_EDI(%edx), %edi
    movl CONTEXT_EBP(%edx), %ebp
    movl CONTEXT_ESP(%edx), %esp
    push CONTEXT_EFLAGS(%edx)
    popfl

    /* Restore address space.*/
    movl CONTEXT_CR3(%edx), %eax
    movl %eax, %cr3

    /* Update ESP0 on TSS. */
    movl CONTEXT_ESP0(%edx), %eax
    movl $TSS , %edx
    movl %eax, TSS_ESP0(%edx)

    __context_switch.out:
    ret

/*----------------------------------------------------------------------------*
 * __leave_kernel()                                                           *
 *----------------------------------------------------------------------------*/

/*
 * Leaves the kernel.
 */
__leave_kernel_to_user_mode:

    /*
     * Restore data segment registers.
     */
    movl 16(%esp), %eax /* eax gets user_ds */
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %fs
    movw %ax, %gs

__leave_kernel:
    iret


/*----------------------------------------------------------------------------*
 * __physcopy()                                                               *
 *----------------------------------------------------------------------------*/

/*
 * Physical memory copy.
 */
__physcopy:
	pushl %esi
	pushl %edi

	/* Get parameters. */
	movl 12(%esp), %edi /* dest */
	movl 16(%esp), %esi /* src */
	movl 20(%esp), %ecx /* size */

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
__physcopy.loop:
	movl (%esi), %eax
	movl %eax, (%edi)
	inc %esi
	inc %edi
	dec %ecx
	jnz __physcopy.loop

  	/* Re-enable paging. */
	movl %cr0, %eax
	orl $0x80000000, %eax
	movl %eax, %cr0

	popl %edi
	popl %esi

    ret

/*----------------------------------------------------------------------------*
 * __has_cpuid()                                                              *
 *----------------------------------------------------------------------------*/


/*
 * Checks if the CPU supports the CPUID instruction.
 */
 .align 4
__has_cpuid:
    pushfl                   # Save EFLAGS
    pushfl                   # Store EFLAGS
    xorl $EFLAGS_ID, (%esp)  # Invert the ID bit in stored EFLAGS
    popfl                    # Load stored EFLAGS (with ID bit inverted)
    pushfl                   # Store EFLAGS again (ID bit may or may not be inverted)
    popl %eax                # eax = modified EFLAGS (ID bit may or may not be inverted)
    xorl (%esp), %eax        # eax = whichever bits were changed
    popfl                    # Restore original EFLAGS
    andl $EFLAGS_ID, %eax    # eax = zero if ID bit can't be changed, else non-zero
    movl $0, %eax
    jz __has_cpuid.out
    movl $1, %eax
    __has_cpuid.out:
    ret

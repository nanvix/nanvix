/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * This file is part of Nanvix.
 *
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

/* Must come first. */
#define _ASM_FILE_

#include <i386/i386.h>
#include <i386/int.h>
#include <nanvix/fs.h>
#include <nanvix/pm.h>
#include <nanvix/syscall.h>
#include <signal.h>
#include <errno.h>

/* Exported symbols. */
.globl swint0
.globl swint1
.globl swint2
.globl swint3
.globl swint4
.globl swint5
.globl swint6
.globl swint7
.globl swint8
.globl swint9
.globl swint10
.globl swint11
.globl swint12
.globl swint13
.globl swint14
.globl swint15
.globl swint16
.globl swint17
.globl syscall
.globl hwint0
.globl hwint1
.globl hwint2
.globl hwint3
.globl hwint4
.globl hwint5
.globl hwint6
.globl hwint7
.globl hwint8
.globl hwint9
.globl hwint10
.globl hwint11
.globl hwint12
.globl hwint13
.globl hwint14
.globl hwint15
.globl leave
.globl do_hwint

/*----------------------------------------------------------------------------*
 *                                  save()                                    *
 *----------------------------------------------------------------------------*/

/*
 * Saves registers.
 */
.macro save
	pushl %eax
	pushl %ecx
	pushl %edx
	pushl %ebx
	pushl %ebp
	pushl %esi
	pushl %edi

	/* Save data segment. */
	pushl %ds
.endm

/*----------------------------------------------------------------------------*
 *                                 enter()                                    *
 *----------------------------------------------------------------------------*/

/*
 * Enters in kernel.
 */
.macro enter
	movw $KERNEL_DS, %bx
	movw %bx, %ds

	movl curr_proc, %ebx

    /* Increment interrupt level. */
    incl PROC_INTLVL(%ebx)

    /* Save interrupt stack. */
    pushl PROC_KESP(%ebx)
    movl %esp, PROC_KESP(%ebx)

.endm

/*----------------------------------------------------------------------------*
 *                                leave()                                     *
 *----------------------------------------------------------------------------*/

/*
 * Leaves kernel.
 */
leave:
	movl curr_proc, %ebx

	/* Restore interrupt stack. */
	popl PROC_KESP(%ebx)

	/* Decrement interrupt level. */
    decl PROC_INTLVL(%ebx)
    jnz leave.out

	/* Check if we were handling a system call. */
	btrl $PROC_SYS, PROC_FLAGS(%ebx)
	jnc check_signals

	/*
	 * The kernel is non-preemptive.
	 * So, let us be nice with other processes
	 * and give them a chance to run.
	 */
	call yield

	movl curr_proc, %ebx

	/* Check signals. */
	check_signals:
		bsfl PROC_RECEIVED(%ebx), %eax
		jz leave.out
		btrl %eax, PROC_RECEIVED(%ebx)
		movl PROC_HANDLERS(%ebx, %eax, 4), %ecx
		cmpl $_SIG_DFL, %ecx
		je default_signal
		cmpl $_SIG_IGN, %ecx
		je check_signals

			/* User signal. */
			movl $_SIG_DFL, PROC_HANDLERS(%ebx, %eax, 4)
			movl PROC_RESTORER(%ebx), %esi
			movl USERESP - 4(%esp), %edx
			pushl %eax
			pushl %ecx
			pushl $MAY_WRITE
			pushl $44
			pushl %edx
			call chkmem
			addl $12, %esp
			pop %ecx
			pop %ebx
			cmpl $0, %eax
			je leave.out

			/* Build stack for signal handler. */
			movl USERESP - 4(%esp), %eax
			movl EIP - 4(%esp), %edx
			movl %edx, -4(%eax)           # eip
			movl EFLAGS - 4(%esp), %edx
			movl %edx, -8(%eax)           # eflags
			movl EAX - 4(%esp), %edx
			movl %edx, -12(%eax)          # eax
			movl ECX - 4(%esp), %edx
			movl %edx, -16(%eax)          # ecx
			movl EDX - 4(%esp), %edx
			movl %edx, -20(%eax)          # edx
			movl EBX - 4(%esp), %edx
			movl %edx, -24(%eax)          # ebx
			movl EBP - 4(%esp), %edx
			movl %edx, -28(%eax)          # ebp
			movl ESI - 4(%esp), %edx
			movl %edx, -32(%eax)          # esi
			movl EDI - 4(%esp), %edx
			movl %edx, -36(%eax)          # edi
			movl %ebx, -40(%eax)          # Signal number.
			movl %esi, -44(%eax)          # Restorer address.

			movl %ecx, EIP - 4(%esp)
			subl $44, USERESP - 4(%esp)

leave.out:
	popl %ds
	popl %edi
	popl %esi
	popl %ebp
	popl %ebx
	popl %edx
	popl %ecx
	popl %eax
	iret

default_signal:
	cmpl $SIGCHLD, %eax
	je leave.out
	pushl %eax
	call *sigdfl(, %eax, 4)
	addl $4, %esp
	jmp leave.out

/*----------------------------------------------------------------------------*
 *                                  swint()                                   *
 *----------------------------------------------------------------------------*/

/*
 * Software interrupt without error code.
 */
.macro swint_noerr_code, number, handler
	swint\()\number:
		pushl %eax
		movl $0, %eax
		pushl %ecx
		movl $\handler, %ecx
		jmp swint
.endm

/*
 * Software interrupt with error code.
 */
.macro swint_err_code, number, handler
	swint\()\number:
		xchg %eax, (%esp)
		pushl %ecx
		movl $\handler, %ecx
		jmp swint
.endm

/* Software interrupt hooks. */
swint_noerr_code  0 do_divide
swint_noerr_code  1 do_debug
swint_noerr_code  2 do_nmi
swint_noerr_code  3 do_breakpoint
swint_noerr_code  4 do_overflow
swint_noerr_code  5 do_bounds
swint_noerr_code  6 do_invalid_opcode
swint_noerr_code  7 do_coprocessor_not_available
swint_err_code    8 do_double_fault
swint_noerr_code  9 do_coprocessor_segment_overrun
swint_err_code   10 do_invalid_tss
swint_err_code   11 do_segment_not_present
swint_err_code   12 do_stack_exception
swint_err_code   13 do_general_protection
swint_err_code   14 _do_page_fault
swint_noerr_code 15 do_reserved
swint_noerr_code 16 do_coprocessor_error
swint_noerr_code 17 do_reserved

/*
 * Software interrupt handler dispatcher.
 */
swint:
	/* Save registers. */
	pushl %edx
	pushl %ebx
	pushl %ebp
	pushl %esi
	pushl %edi
	pushl %ds

	enter

	/* Leave critical region. */
	sti

	/* Call interrupt handler. */
	pushl %eax
	call *%ecx
	add $4, %esp

	/* Enter critical region. */
	cli

    jmp leave

/*
 * Page fault handler wrapper.
 */
_do_page_fault:
	pushl 4(%esp)
	movl %cr2, %eax
	pushl %eax
	call do_page_fault
	addl $8, %esp
	ret

/*----------------------------------------------------------------------------*
 *                                 syscall()                                  *
 *----------------------------------------------------------------------------*/

/*
 * System call hook.
 */
syscall:
	save
	enter

	/* Set 'handling system call' flag. */
	btsl $PROC_SYS, PROC_FLAGS(%ebx)

	/* Leave critical region. */
	sti

	/* Get system call parameters. */
	movl EAX(%esp), %eax
	movl EBX(%esp), %ebx
	movl ECX(%esp), %ecx
	movl EDX(%esp), %edx

	/* Check for bad system call. */
	cmpl $NR_SYSCALLS, %eax
	jl good_syscall
		movl $ENOSYS, %eax
		negl %eax
	jmp bad_syscall

	good_syscall:
		/* Get system call. */
		shl $2, %eax
		addl $syscalls_table, %eax
		movl (%eax), %eax

		/* Do system call. */
		pushl %edx
		pushl %ecx
		pushl %ebx
		call *%eax
		addl $12, %esp
	bad_syscall:

	/* Copy return value to user stack. */
	movl %eax, EAX(%esp)

	/* Enter critical region. */
	cli

	jmp leave

/*----------------------------------------------------------------------------*
 *                                   hwint()                                  *
 *----------------------------------------------------------------------------*/

/*
 * Master hardware interrupt.
 */
.macro hwint_master, num
	hwint\()\num:
		save
		enter
		movb $0x20, %al
		outb %al, $PIC_CTRL_MASTER /* Send reset signal to master. */
		pushl $(\num)
		call do_hwint
		addl $4, %esp
		jmp leave
.endm

/*
 * Slave hardware interrupt.
 */
.macro hwint_slave, num
	hwint\()\num:
		save
		enter
		movb $0x20, %al
		outb %al, $PIC_CTRL_MASTER /* Send reset signal to master. */
		outb %al, $PIC_CTRL_SLAVE  /* Send reset signal to slave.  */
		pushl $(\num)
		call do_hwint
		addl $4, %esp
		jmp leave
.endm

/* Hardware interrupt hooks. */
hwint_master 0
hwint_master 1
hwint_master 2
hwint_master 3
hwint_master 4
hwint_master 5
hwint_master 6
hwint_master 7
hwint_slave  8
hwint_slave  9
hwint_slave 10
hwint_slave 11
hwint_slave 12
hwint_slave 13
hwint_slave 14
hwint_slave 15

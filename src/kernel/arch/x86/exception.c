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

#include <i386/int.h>
#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <signal.h>

/*
 * Exception handler.
 */
#define EXCEPTION(name, sig, msg)                                   \
PUBLIC void do_##name(int err, struct intstack s)                   \
{                                                                   \
	/* Die. */                                                      \
	if (KERNEL_RUNNING(curr_proc))                                  \
	{                                                               \
		kprintf("%s: %d", msg, err & 0xffff);                       \
		dumpregs(&s);                                               \
		kprintf("process %d caused and exception", curr_proc->pid); \
		kpanic("kernel running");                                   \
	}                                                               \
	                                                                \
	sndsig(curr_proc, sig);                                         \
}                                                                   \

/*
 * Dump kernel registers.
 */
PRIVATE void dumpregs(struct intstack *regs)
{
	/* Dump registers. */
	kprintf("  [eax: %x] [ebx:    %x]", regs->eax, regs->ebx);
	kprintf("  [ecx: %x] [edx:    %x]", regs->ecx, regs->edx);
	kprintf("  [esi: %x] [edi:    %x]", regs->esi, regs->edi);
	kprintf("  [ebp: %x] [esp:    %x]", regs->ebp, curr_proc->kesp);
	kprintf("  [eip: %x] [eflags: %x]", regs->eip, regs->eflags);
}

/* Easy-to-handle exceptions. */
EXCEPTION(divide,                      SIGFPE,  "divide error")
EXCEPTION(breakpoint,                  SIGTRAP, "breakpoint exception")
EXCEPTION(overflow,                    SIGSEGV, "overflow exception")
EXCEPTION(bounds,                      SIGSEGV, "bounds check exception")
EXCEPTION(invalid_opcode,              SIGILL,  "invalid opcode exception")
EXCEPTION(coprocessor_not_available,   SIGSEGV, "coprocessor not available")
EXCEPTION(double_fault,                SIGSEGV, "double fault")
EXCEPTION(coprocessor_segment_overrun, SIGFPE,  "coprocessor segment overrun")
EXCEPTION(invalid_tss,                 SIGSEGV, "invalid tss")
EXCEPTION(segment_not_present,         SIGSEGV, "segment not present")
EXCEPTION(stack_exception,             SIGSEGV, "stack exception")
EXCEPTION(general_protection,          SIGSEGV, "general protection fault")
EXCEPTION(reserved,                    SIGSEGV, "reserved exception")
EXCEPTION(coprocessor_error,           SIGSEGV, "coprocessor error")

/*
 * Handles a non maskable interrupt.
 */
PUBLIC void do_nmi(void)
{
	kprintf("nmi received but trying to continue");
}

/*
 * Handles a debug exception.
 */
PUBLIC void do_debug(void)
{
	kprintf("debug exception received but not implemented");
}

/*
 * Handles a page fault.
 */
PUBLIC void do_page_fault(addr_t addr, int err, int dummy0, int dummy1, struct intstack s)
{
	((void)dummy0);
	((void)dummy1);

	/* Validty page fault. */
	if (!(err & 1))
	{
		if (!vfault(addr))
			return;
	}

	/* Protection page fault. */
	if (err & 2)
	{
		if (!pfault(addr))
			return;
	}

	if (KERNEL_RUNNING(curr_proc))
	{
		dumpregs(&s);
		kpanic("kernel page fault %d at %x", err, addr);
	}

	sndsig(curr_proc, SIGSEGV);
	die(((SIGSEGV & 0xff) << 16) | (1 << 9));
}

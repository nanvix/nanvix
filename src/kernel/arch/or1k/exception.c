/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2018-2018 Davidson Francis <davidsondfgl@gmail.com>
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

#include <or1k/int.h>
#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/smp.h>
#include <signal.h>

/*
 * Exception handler.
 */
#define EXCEPTION(name, sig, msg)                                   \
PUBLIC void do_##name(int err, struct intstack s)                   \
{                                                                   \
	/* Die. */                                                      \
	if (KERNEL_WAS_RUNNING(cpus[curr_core].curr_thread))            \
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
	kprintf("  [r1: %x] [r9 : %x] [r17: %x] [r25:  %x] [EEAR: %x]",
		regs->gpr[1], regs->gpr[9], regs->gpr[17], regs->gpr[25], regs->eear);
	kprintf("  [r2: %x] [r10: %x] [r18: %x] [r26:  %x] [ESR:  %x]",
		regs->gpr[2], regs->gpr[10], regs->gpr[18], regs->gpr[26], regs->esr);
	kprintf("  [r3: %x] [r11: %x] [r19: %x] [r27:  %x]",
		regs->gpr[3], regs->gpr[11], regs->gpr[19], regs->gpr[27]);
	kprintf("  [r4: %x] [r12: %x] [r20: %x] [r28:  %x]",
		regs->gpr[4], regs->gpr[12], regs->gpr[20], regs->gpr[28]);
	kprintf("  [r5: %x] [r13: %x] [r21: %x] [r29:  %x]",
		regs->gpr[5], regs->gpr[13], regs->gpr[21], regs->gpr[29]);
	kprintf("  [r6: %x] [r14: %x] [r22: %x] [r30:  %x]",
		regs->gpr[6], regs->gpr[14], regs->gpr[22], regs->gpr[30]);
	kprintf("  [r7: %x] [r15: %x] [r23: %x] [r31:  %x]",
		regs->gpr[7], regs->gpr[15], regs->gpr[23], regs->gpr[31]);
	kprintf("  [r8: %x] [r16: %x] [r24: %x] [EPCR: %x]",
		regs->gpr[8], regs->gpr[16], regs->gpr[24], regs->epcr);
}

/* Easy-to-handle exceptions. */
EXCEPTION(reset,               SIGBUS,  "reset exception")
EXCEPTION(bus_error,           SIGSEGV, "bus error")
EXCEPTION(alignment,           SIGBUS,  "aligment exception")
EXCEPTION(illegal_instruction, SIGILL,  "illegal instruction")
EXCEPTION(external_interrupt,  SIGSEGV, "external interrupt")
EXCEPTION(range,               SIGSEGV, "range exception")
EXCEPTION(float,               SIGFPE,  "floating point exception")
EXCEPTION(trap,                SIGTRAP, "trap")
EXCEPTION(reserved,            SIGSEGV, "reserved exception")

/*
 * Handles a page fault.
 */
PUBLIC void do_page_fault(addr_t addr, int err, int dummy0, int dummy1, struct intstack s)
{	
	((void)dummy0);
	((void)dummy1);
	
	/* Validity page fault. */
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
	
	if (KERNEL_WAS_RUNNING(cpus[curr_core].curr_thread))
	{
		dumpregs(&s);
		kpanic("kernel page fault %d at %x", err, addr);
	}
	
	sndsig(curr_proc, SIGSEGV);
	die(((SIGSEGV & 0xff) << 16) | (1 << 9));
}

/*
 * Copyright (C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * core/exception.c - Exception handlers.
 */

#include <nanvix/const.h>
#include <nanvix/int.h>
#include <nanvix/klib.h>
#include <nanvix/paging.h>
#include <signal.h>

/*
 * Exception handler.
 */
#define EXCEPTION(name, sig, msg)                  \
PUBLIC void do_##name(int err, struct intstack s)  \
{                                                  \
	/* Die. */                                     \
	if (!KERNEL_RUNNING(curr_proc))                \
	{                                              \
		kprintf("%s: %d", msg, err & 0xffff);      \
		dumpregs(&s);                              \
		kpanic("kernel caused an exception");      \
	}                                              \
	                                               \
	sndsig(curr_proc, sig);                        \
}                                                  \

/*
 * Kills process.
 */
PRIVATE void dumpregs(struct intstack *regs)
{
	/* Dump registers. */
	kprintf("  [eax: %x] [ebx:    %x]", regs->eax, regs->ebx);
	kprintf("  [ecx: %x] [edx:    %x]", regs->ecx, regs->edx);
	kprintf("  [esi: %x] [edi:    %x]", regs->esi, regs->edi);
	kprintf("  [ebp: %x]", regs->ebp);
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
PUBLIC void do_page_fault(addr_t addr, int err)
{	
	kprintf("page fault %d at %x", err, addr);
	
	/* Validty page fault. */
	if (!(err & 1))
	{
		vfault(addr);
		return;
	}
	
	/* Protection page fault. */
	if (err & 2)
	{
		pfault(addr);
		return;
	}
	
	kpanic("kernel page fault");
}

/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * util.h - Low-level utilities
 */

#ifndef UTIL_H_
#define UTIL_H_

	#include <i386/i386.h>
	#include <nanvix/const.h>
	#include <nanvix/pm.h>
	#include <sys/types.h>	
	
	/* CPU levels. */
	#define CPULVL_CLOCK    (0xff)
	#define CPULVL_DISK     (~((1 <<INT_CLOCK) | (1 <<INT_CMOS)) & CPULVL_CLOCK)
	#define CPULVL_TERMINAL (~((1 <<INT_ATA1) | (1 <<INT_ATA2)) & CPULVL_DISK)
	#define CPULVL_NONE     (0x00)

	/*
	 * DESCRIPTION:
	 *   The gdt_flush() function flushes the GDT pointed to by gdtptr.
	 * 
	 * RETURN VALUE:
	 *   The gdt_flush() function returns no value.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void gdt_flush(struct gdtptr *gdtptr);
	
	/*
	 * DESCRIPTION:
	 *   The tss_flush() function flushes the TSS.
	 * 
	 * RETURN VALUE:
	 *   The tss_flush() function returns no value.
	 * 
	 * ERRORS:
	 *   No erros are defined.
	 */
	EXTERN void tss_flush();
	
	/*
	 * DESCRIPTION:
	 *   The idt_flush() function flushes the IDT pointed to by idtptr.
	 * 
	 * RETURN VALUE:
	 *   The idt_flush() function returns no value.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void idt_flush(struct idtptr *idtptr);
	
	/*
	 * DESCRIPTION:
	 *   The enable_interrupt() function enables all hardware interrupts.
	 *
	 * RETURN VALUE:
	 *   The enable_interrupt() function has no return value.
	 * 
	 * ERRORS:
	 *   No erros are defined.
	 */
	EXTERN void enable_interrupts();
	
	/*
	 * DESCRIPTION:
	 *   The disable_interrupts() function disables all hardware interrupts.
	 * 
	 * RETURN VALUE:
	 *   The disable_interrupts() function has no return value.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void disable_interrupts();
	
	/*
	 * DESCRIPTION:
	 *   The halt() function halts the processor.
	 * 
	 * RETURN VALUE:
	 *   The halt() function has no return value.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void halt();
	
	/*
	 * DESCRIPTION:
	 *   The physcpy() function performs a physical memory copy of n bytes from
	 *   the source address src to the destin address dest.
	 * 
	 * RETURN VALUE:
	 *   The physcpy() function has no return value.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void physcpy(addr_t dest, addr_t src, size_t n);
	
	/*
	 * 
	 */
	EXTERN void switch_to(struct process *proc);
	
	/*
	 * Changes the CPU level.
	 */
	EXTERN uint16_t cpulvl(uint16_t lvl);
	
#endif /* UTIL_H_ */

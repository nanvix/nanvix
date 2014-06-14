/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * <asm/util.h> - Low-level utilities.
 */

#ifndef UTIL_H_
#define UTIL_H_

	#include <i386/i386.h>
	#include <nanvix/const.h>
	#include <nanvix/pm.h>
	#include <sys/types.h>

	/*
	 * Flushes the GDT pointed to by gdtptr.
	 */
	EXTERN void gdt_flush(struct gdtptr *gdtptr);
	
	/*
	 * Flushes the TSS.
	 */
	EXTERN void tss_flush(void);
	
	/*
	 * Flushes the TLB.
	 */
	EXTERN void tlb_flush(void);
	
	/*
	 * Flushes the IDT pointed to by idtptr.
	 */
	EXTERN void idt_flush(struct idtptr *idtptr);
	
	/*
	 * Enables all hardware interrupts.
	 */
	EXTERN void enable_interrupts(void);
	
	/*
	 * Disables all hardware interrupts.
	 */
	EXTERN void disable_interrupts(void);
	
	/*
	 * Halts the processor.
	 */
	EXTERN void halt(void);
	
	/*
	 * Performs a physical memory copy.
	 */
	EXTERN void physcpy(addr_t dest, addr_t src, size_t n);
	
	/*
	 * Switches execution to a process.
	 */
	EXTERN void switch_to(struct process *proc);
	
	/*
	 * Switches to user mode.
	 */
	EXTERN void user_mode(addr_t entry, addr_t sp);
	
#endif /* UTIL_H_ */

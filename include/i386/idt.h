/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * idt.h - Interrupt descriptor table
 */

#ifndef IDT_H_
#define IDT_H_

	/* IDT entry size (in bytes). */
	#define IDTE_SIZE 8
	
	/* IDT pointer size (in bytes). */
	#define IDTPTR_SIZE 6
	
	/* IDT size (number of entries). */
	#define IDT_SIZE 256

	/* IDT gate types. */
	#define IDT_TASK32 0x5 /* 32-bit task gate.      */
	#define IDT_INT16  0x6 /* 16-bit interrupt gate. */
	#define IDT_TRAP16 0x7 /* 16-bit trap gate.      */
	#define IDT_INT32  0xe /* 32-bit interrupt gate. */
	#define IDT_TRAP32 0xf /* 32-bit trap gate.      */

#ifndef _ASM_FILE_

	/*
	 * Interrupt descriptor table entry.
	 */
	struct idte
	{
		unsigned handler_low  : 16; /* Handler low.           */
		unsigned selector     : 16; /* GDT selector.          */
		unsigned              :  8; /* Always zero.           */
		unsigned type         :  4; /* Gate type (sse above). */
		unsigned flags        :  4; /* Flags.                 */
		unsigned handler_high : 16; /* handler high.          */
	} __attribute__((packed));
	
	/*
	 * Interrupt descriptor table pointer.
	 */
	struct idtptr
	{
		unsigned size : 16; /* IDT size.            */
		unsigned ptr  : 32; /* IDT virtual address. */
	} __attribute__((packed));

#endif /* _ASM_FILE_ */

#endif /* IDT_H_ */

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

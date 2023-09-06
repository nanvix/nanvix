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

#ifndef I386_H_
#define I386_H_

	#include <i386/gdt.h>
	#include <i386/idt.h>
	#include <i386/paging.h>
	#include <i386/8259.h>
	#include <i386/pit.h>
	#include <i386/tss.h>
	#include <stdint.h>

	/* Size of machine types. */
	#define BYTE_SIZE  1 /* 1 byte.  */
	#define WORD_SIZE  2 /* 2 bytes. */
	#define DWORD_SIZE 4 /* 4 bytes. */
	#define QWORD_SIZE 8 /* 8 bytes. */

	/* BIt-legth of machine types. */
	#define BYTE_BIT    8 /* 8 bits.  */
	#define WORD_BIT   16 /* 16 bits. */
	#define DWORD_BIT  32 /* 32 bits. */
	#define QWORD_BIT  64 /* 64 bits. */

	/* Offsets to the jmp_buf structure*/
	#define JMP_BUF_EBX     0
	#define JMP_BUF_ESI     4
	#define JMP_BUF_EDI     8
	#define JMP_BUF_EFLAGS 12
	#define JMP_BUF_EBP    16
	#define JMP_BUF_ESP    20
	#define JMP_BUF_EIP    24	/* eip must be last */
	#define JMP_BUF_KESP   28	/* don't know if this should be saved */
	#define JMP_BUF_INTLVL 32	/* don't know if this should be saved */

#ifndef _ASM_FILE_

	/* Machine types. */
	typedef uint8_t byte_t;   /* Byte.        */
	typedef uint16_t word_t;  /* Word.        */
	typedef uint32_t dword_t; /* Double word. */

	/* Used for addresses. */
	typedef unsigned addr_t;

	/*
	 * Converts to address.
	 */
	#define ADDR(x) ((addr_t)(x))

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

#endif /* _ASM_FILE_ */

#endif /* I386_H_ */

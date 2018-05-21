/*
 * Copyright(C) 2017 Davidson Francis <davidsondfgl@gmail.com>
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

/* Copyright (c) 2014 Authors
 *
 * Contributor Julius Baxter <julius.baxter@orsoc.se>
 * Contributor Stefan Wallentowitz <stefan.wallentowitz@tum.de>
 *
 * The authors hereby grant permission to use, copy, modify, distribute,
 * and license this software and its documentation for any purpose, provided
 * that existing copyright notices are retained in all copies and that this
 * notice is included verbatim in any distributions. No written agreement,
 * license, or royalty fee is required for any of the authorized uses.
 * Modifications to this software may be copyrighted by their authors
 * and need not follow the licensing terms described here, provided that
 * the new terms are clearly indicated on the first page of each file where
 * they apply.
 */

#ifndef OR1K_H_
#define OR1K_H_

	#include <or1k/paging.h>
	#include <or1k/spr_defs.h>
	#include <or1k/8259.h>
	#include <stdint.h>

	/**
	 * Access byte-sized memory mapped register
	 *
	 * Used to access a byte-sized memory mapped register. It avoids usage errors
	 * when not defining register addresses volatile and handles casting correctly.
	 *
	 * Example for both read and write:
	 *
	 * \code
	 * uint8_t status = REG8(IPBLOCK_STATUS_REG_ADDR);
	 * REG8(IPBLOCK_ENABLE) = 1;
	 * \endcode
	 *
	 * @param add Register address
	 */
	#define REG8(add) *((volatile unsigned char *) (add))

	/**
	 * Access halfword-sized memory mapped register
	 *
	 * Used to access a 16 byte-sized memory mapped register. It avoids usage errors
	 * when not defining register addresses volatile and handles casting correctly.
	 *
	 * See REG8() for an example.
	 *
	 * @param add Register address
	 */
	#define REG16(add) *((volatile unsigned short *) (add))

	/**
	 * Access word-sized memory mapped register
	 *
	 * Used to access a word-sized memory mapped register. It avoids usage errors
	 * when not defining register addresses volatile and handles casting correctly.
	 *
	 * See REG8() for an example.
	 *
	 * @oaran add Register address
	 */
	#define REG32(add) *((volatile unsigned long *) (add))

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
	 * Flushes the TLB.
	 */
	EXTERN void tlb_flush(void);

	/*
	 * Move from Special-Purpose Register.
	 */
	EXTERN uint32_t mfspr(uint32_t spr);

	/*
	 * Move to Special-Purpose Register.
	 */
	EXTERN void mtspr(uint32_t spr, uint32_t val);

#endif /* _ASM_FILE_ */

#endif /* OR1K_H_ */

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

#ifndef TSS_H_
#define TSS_H_

	/* TSS size (in bytes). */
	#define TSS_SIZE 104

	/* Offsets to the tss structure. */
	#define TSS_ESP0     4 /* Ring 0 stack pointer.         */
	#define TSS_SS0      8 /* Ring 0 stack segment.         */
	#define TSS_ESP1    12 /* Ring 1 stack pointer.         */
	#define TSS_SS1     16 /* Ring 1 stack segment.         */
	#define TSS_ESP2    20 /* Ring 2 stack pointer.         */
	#define TSS_SS2     24 /* Ring 2 stack segment.         */
	#define TSS_CR3     28 /* cr3.                          */
	#define TSS_EIP     32 /* eip.                          */
	#define TSS_EFLAGS  36 /* eflags.                       */
	#define TSS_EAX     40 /* eax.                          */
	#define TSS_ECX     44 /* ecx.                          */
	#define TSS_EDX     48 /* edx.                          */
	#define TSS_EBX     52 /* ebx.                          */
	#define TSS_ESP     56 /* esp.                          */
	#define TSS_EBP     60 /* ebp.                          */
	#define TSS_ESI     64 /* esi.                          */
	#define TSS_EDI     68 /* edi.                          */
	#define TSS_ES      72 /* es.                           */
	#define TSS_CS      76 /* cs.                           */
	#define TSS_SS      80 /* ss.                           */
	#define TSS_DS      84 /* ds.                           */
	#define TSS_FS      88 /* fs.                           */
	#define TSS_GS      92 /* gs.                           */
	#define TSS_LDTR    96 /* LDT selector.                 */
	#define TSS_IOMAP  100 /* IO map.                       */

#ifndef _ASM_FILE_

	/*
	 * Task state segment.
	 */
	struct tss
	{
		unsigned link;   /* Previous TSS in the TSS list. */
		unsigned esp0;   /* Ring 0 stack pointer.         */
		unsigned ss0;    /* Ring 0 stack segment.         */
		unsigned esp1;   /* Ring 1 stack pointer.         */
		unsigned ss1;    /* Ring 1 stack segment.         */
		unsigned esp2;   /* Ring 2 stack pointer.         */
		unsigned ss2;    /* Ring 2 stack segment.         */
		unsigned cr3;    /* cr3.                          */
		unsigned eip;    /* eip.                          */
		unsigned eflags; /* eflags.                       */
		unsigned eax;    /* eax.                          */
		unsigned ecx;    /* ecx.                          */
		unsigned edx;    /* edx.                          */
		unsigned ebx;    /* ebx.                          */
		unsigned esp;    /* esp.                          */
		unsigned ebp;    /* ebp.                          */
		unsigned esi;    /* esi.                          */
		unsigned edi;    /* edi.                          */
		unsigned es;     /* es.                           */
		unsigned cs;     /* cs.                           */
		unsigned ss;     /* ss.                           */
		unsigned ds;     /* ds.                           */
		unsigned fs;     /* fs.                           */
		unsigned gs;     /* gs.                           */
		unsigned ldtr;   /* LDT selector.                 */
		unsigned iomap;  /* IO map.                       */
	} __attribute__((packed));

#endif /* _ASM_FILE_ */

#endif /* TSS_H_ */

/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * i386.h - Intel 80386 specification
 */

#ifndef I386_H_
#define I386_H_

	#include <i386/gdt.h>
	#include <i386/idt.h>
	#include <i386/int.h>
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
	#define JMP_BUF_EIP    12
	#define JMP_BUF_EFLAGS 16
	#define JMP_BUF_EBP    20
	#define JMP_BUF_ESP    24
	#define JMP_BUF_KESP   28
	#define JMP_BUF_INTLVL 32
	
#ifndef _ASM_FILE_

	/* Machine types. */
	typedef uint8_t byte_t;   /* Byte.        */
	typedef uint16_t word_t;  /* Word.        */
	typedef uint32_t dword_t; /* Double word. */
	
	/* Used for addresses. */
	typedef unsigned addr_t;

	typedef struct
	{
		dword_t ebx;
		dword_t esi;
		dword_t edi;
		dword_t eip;
		dword_t eflags;
		dword_t ebp;
		dword_t esp;
		dword_t kesp;
		dword_t intlvl;
	} kjmp_buf;

	/*
	 * Converts to address.
	 */
	#define ADDR(x) ((addr_t)addr)

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

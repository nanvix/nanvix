/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * int.h - Interrupt library
 */

#ifndef INT_H_
#define INT_H_

	#include <i386/i386.h>
	#include <nanvix/const.h>
	#include <errno.h>

	/* Hardware interrupt numbers. */
	#define HWINT_CLOCK    0 /* Clock interrupt. */

	/* Offsets to the registers structure. */
	#define DS       4
	#define EDI      8
	#define ESI     12
	#define EBP     16
	#define EBX     20
	#define EDX     24
	#define ECX     28
	#define EAX     32
	#define EIP     36
	#define CS      40
	#define EFLAGS  44
	#define USERESP 48
	#define SS      52

#ifndef _ASM_FILE_

	/*
	 * Saved registers durint interrupt/exception.
	 */
	struct intstack
	{   
		dword_t old_kesp;
        dword_t ds;
        dword_t edi, esi, ebp, ebx, edx, ecx, eax;
        dword_t eip, cs, eflags, useresp, ss;	
	};

	/* Software interrupt hooks. */
	EXTERN void swint0();
	EXTERN void swint1();
	EXTERN void swint2();
	EXTERN void swint3();
	EXTERN void swint4();
	EXTERN void swint5();
	EXTERN void swint6();
	EXTERN void swint7();
	EXTERN void swint8();
	EXTERN void swint9();
	EXTERN void swint10();
	EXTERN void swint11();
	EXTERN void swint12();
	EXTERN void swint13();
	EXTERN void swint14();
	EXTERN void swint15();
	EXTERN void swint16();
	EXTERN void swint17();
	EXTERN void swint19();
	
	/* System call hook. */
	EXTERN void syscall();
	
	/* Hardware interrupt hooks. */
	EXTERN void hwint0();
	EXTERN void hwint1();
	EXTERN void hwint2();
	EXTERN void hwint3();
	EXTERN void hwint4();
	EXTERN void hwint5();
	EXTERN void hwint6();
	EXTERN void hwint7();
	EXTERN void hwint8();
	EXTERN void hwint9();
	EXTERN void hwint10();
	EXTERN void hwint11();
	EXTERN void hwint12();
	EXTERN void hwint13();
	EXTERN void hwint14();
	EXTERN void hwint15();
	EXTERN void hwint16();
	
	/*
	 * DESCRIPTION:
	 *   The set_hwint() function sets a handler pointer to by handler for the 
	 *   hardware interrupt identified by num.
	 * 
	 * RETURN VALUE:
	 *   Upon success, the set_hwint() function returns 0. Upon failure, a  
	 *   negative error code is returned.
	 * 
	 * ERRORS:
	 *   - EBUSY: there is a handler set to the hardware interrupt specified.
	 */
	EXTERN int set_hwint(int num, void (*handler)(void));

#endif /* _ASM_FILE */

#endif /* INT_H_ */

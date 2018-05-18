/*
 * Copyright(C) 2011-2018 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2018-2018 Davidson Francis <davidsondfgl@gmail.com>
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

#ifndef OR1K_INT_H_
#define OR1K_INT_H_

	#include <or1k/or1k.h>

	/* Interrupt numbers. */
	#define INT_KEYBOARD 1 /* Keyboard.                                  */
	#define INT_ATA1    14 /* Primary ATA hard disk.                     */
	#define INT_ATA2    15 /* Secondary ATA hard disk.                   */

	#define INT_CLOCK      0 /* Timer.                        */
	#define INT_COM1       2 /* COM1.                         */
	#define INT_EXTERNAL 256 /* External interrupt indicator. */

	/* Offsets to the registers structure. */
	#define R0           4
	#define SP           8
	#define GPR2        12
	#define GPR3        16
	#define GPR4        20
	#define GPR5        24
	#define GPR6        28
	#define GPR7        32
	#define GPR8        36
	#define GPR9        40
	#define GPR10       44
	#define GPR11       48
	#define GPR12       52
	#define GPR13       56
	#define GPR14       60
	#define GPR15       64
	#define GPR16       68
	#define GPR17       72
	#define GPR18       76
	#define GPR19       80
	#define GPR20       84
	#define GPR21       88
	#define GPR22       92
	#define GPR23       96
	#define GPR24      100
	#define GPR25      104
	#define GPR26      108
	#define GPR27      112
	#define GPR28      116
	#define GPR29      120
	#define GPR30      124
	#define GPR31      128
	#define EPCR       132
	#define EEAR       136
	#define ESR        140
	
	/* Stack frame size. */
	#define INT_FRAME_SIZE 144

#ifndef _ASM_FILE_

	/*
	 * Saved registers during interrupt/exception.
	 */
	struct intstack
	{
		dword_t old_kesp;
		dword_t gpr[32];
		dword_t epcr;
		dword_t eear;
		dword_t esr;
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

#endif /* _ASM_FILE */

#endif /* OR1K_INT_H_ */

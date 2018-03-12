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
	#define INT_CLOCK    0 /* Programmable interrupt timer.              */
	#define INT_KEYBOARD 1 /* Keyboard.                                  */
	#define INT_COM2     3 /* COM2.                                      */
	#define INT_COM1     4 /* COM1.                                      */
	#define INT_LPT2     5 /* LPT2.                                      */
	#define INT_FLOPPY   6 /* Floppy disk.                               */
	#define INT_LPT1     7 /* LPT1.                                      */
	#define INT_CMOS     8 /* CMOS real-time clock.                      */
	#define INT_SCSI1    9 /* Free for peripherals (legacy SCSI or NIC). */
	#define INT_SCSI2   10 /* Free for peripherals (legacy SCSI or NIC). */
	#define INT_SCSI3   11 /* Free for peripherals (legacy SCSI or NIC). */
	#define INT_MOUSE   12 /* PS2 mouse.                                 */
	#define INT_COPROC  13 /* FPU, coprocessor or inter-processor.       */
	#define INT_ATA1    14 /* Primary ATA hard disk.                     */
	#define INT_ATA2    15 /* Secondary ATA hard disk.                   */

	/* Offsets to the registers structure. */
	#define SR           0
	#define SP           4
	#define GPR2         8
	#define GPR3        12
	#define GPR4        16
	#define GPR5        20
	#define GPR6        24
	#define GPR7        28
	#define GPR8        32
	#define GPR9        36
	#define GPR10       40
	#define GPR11       44
	#define GPR12       48
	#define GPR13       52
	#define GPR14       56
	#define GPR15       60
	#define GPR16       64
	#define GPR17       68
	#define GPR18       72
	#define GPR19       76
	#define GPR20       80
	#define GPR21       84
	#define GPR22       88
	#define GPR23       92
	#define GPR24       96
	#define GPR25      100
	#define GPR26      104
	#define GPR27      108
	#define GPR28      112
	#define GPR29      116
	#define GPR30      120
	#define GPR31      124
	#define EPCR       128
	#define EEAR       132
	#define ESR        136
	#define SYSCALLNO  140
	
	/* Stack frame size. */
	#define INT_FRAME_SIZE 144

#ifndef _ASM_FILE_

	/*
	 * Saved registers during interrupt/exception.
	 */
	struct intstack
	{
		dword_t gpr[32];
		dword_t pc;
		dword_t eear;
		dword_t esr;
		dword_t syscallno;
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

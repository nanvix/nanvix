/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * int.h - Interrupt library
 */

#ifndef I386_INT_H_
#define I386_INT_H_

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

#endif /* I386_INT_H_ */

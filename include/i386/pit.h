/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * pit.h - Programmable interrupt timer.
 */

#ifndef I386_PIT_H_
#define I386_PIT_H_

	/* Oscillator frequency. */
	#define PIT_FREQUENCY 1193180
	
	/* Registers. */
	#define PIT_CTRL 0x43 /* Control. */
	#define PIT_DATA 0x40 /* Data.    */

#endif /* I386_PIT_H_ */

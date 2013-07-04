/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * pic.h - Programmable interrupt controller
 */

#ifndef PIC_H_
#define PIC_H_

	/* Master PIC. */
	#define PIC_CTRL_MASTER 0x20 /* Control register. */
	#define PIC_DATA_MASTER 0x21 /* Data register.    */
	
	/* Slave PIC. */
	#define PIC_CTRL_SLAVE 0xa0 /* Control register. */
	#define PIC_DATA_SLAVE 0xa1 /* Data register.    */

#endif /* PIC */


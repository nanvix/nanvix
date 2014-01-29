/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * tty.h - tty device driver
 */

#ifndef _TTY_H_
#define _TTY_H_

	#include <nanvix/const.h>
	#include <nanvix/klib.h>
	#include <nanvix/pm.h>
	#include <termios.h>

	/* Console colors. */
    #define BLACK         0x00
    #define BLUE          0x01
    #define GREEN         0x02
    #define CYAN          0x03
    #define RED           0x04
    #define MAGNETA       0x05
    #define BROWN         0x06
    #define LIGHT_GREY    0x07
    #define DARK_GREY     0x08
    #define LIGHT_BLUE    0x09
    #define LIGHT_GREEN   0x0A
    #define LIGHT_CYAN    0x0B
    #define LIGHT_RED     0x0C
    #define LIGHT_MAGNETA 0x0D
    #define LIGHT_BROWN   0x0E
    #define WHITE         0x0F

	/*
	 * tty device.
	 */
	struct tty
	{
		struct termios term;   /* Terminal I/O.  */
		struct process *pgrp;  /* Process group. */
		struct kbuffer output; /* Output buffer. */
		struct kbuffer input;  /* Input buffer.  */
	};
	
	/*
	 * Outputs a colored ASCII character on the console device.
	 */
	EXTERN void console_put(uint8_t ch, uint8_t color);
	
	/*
	 * Initializes the console driver.
	 */
	EXTERN void console_init();
	
	/*
	 * Clears the console.
	 */
	EXTERN void console_clear(void);
	
	/*
	 * Initializes the keyboard driver.
	 */
	EXTERN void keyboard_init(void);
	
	/*
	 * Flushes a buffer on the console device.
	 */
	EXTERN void console_write(struct kbuffer *buffer);
	
	/*
	 * tty device.
	 */
	EXTERN struct tty tty;

#endif /* _TTY_H_ */

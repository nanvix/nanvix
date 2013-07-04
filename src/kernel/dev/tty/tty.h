/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * tty.h - tty device driver
 */

#ifndef _TTY_H_
#define _TTY_H_

	#include <nanvix/const.h>
	#include <nanvix/klib.h>

	/*
	 * tty device.
	 */
	struct tty
	{
		struct kbuffer output; /* Output buffer. */
		struct kbuffer input;  /* Input buffer.  */
	};
	
	/*
	 * Initializes the console driver.
	 */
	EXTERN void console_init();
	
	/*
	 * Flushes a buffer on the console device.
	 */
	EXTERN void console_write(struct kbuffer *buffer);
	
	/*
	 * tty device.
	 */
	EXTERN struct tty tty;

#endif /* _TTY_H_ */

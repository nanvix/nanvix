/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * tty.h - tty device driver
 */

#ifndef TTY_H_
#define TTY_H_

	/* tty ioctl() commands. */
	#define TTY_GETS 0x54000101 /* Get tty settings. */

	/*
	 * Initializes the tty device driver.
	 */
	extern void tty_init(void);

#endif /* TTY_H_ */

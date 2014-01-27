/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * tty.c - tty device driver
 */

#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/klib.h>
#include <nanvix/pm.h>
#include <errno.h>
#include <termios.h>
#include "tty.h"

/*
 * tty device.
 */
PUBLIC struct tty tty;

/*
 * Puts process to sleep if tty output buffer is full.
 */
PRIVATE void sleep_full()
{
	/* Sleep while output buffer is full. */
	while (KBUFFER_FULL(tty.output))
		sleep(&tty.output.chain, PRIO_TTY);
}

/*
 * Puts the calling process to sleep it the tty input buffer is full.
 */
PRIVATE int sleep_empty(void)
{
	/* Sleep while input buffer is empty. */
	while (KBUFFER_EMPTY(tty.input))
	{
		sleep(&tty.input.chain, PRIO_TTY);
		
		/* Awaken by signal. */
		if (issig())
		{
			curr_proc->errno = -EINTR;
			return (-1);
		}
	}
	
	return (0);
}

/*
 * Writes to the tty device.
 */
PRIVATE ssize_t tty_write(unsigned minor, const char *buf, size_t n)
{	
	const char *p;
	
	UNUSED(minor);
	
	p = buf;
	
	/* Write n characters. */
	while (n > 0)
	{
		sleep_full();
		
		/* Copy data to output tty buffer. */
		while ((n > 0) && (!KBUFFER_FULL(tty.output)))
		{
			KBUFFER_PUT(tty.output, *p);
			
			p++, n--;
		}
		
		/* Flushes tty output buffer. */
		console_write(&tty.output);
	}
		
	return ((ssize_t)(p - buf));
}

/*
 * Reads from a tty device.
 */
PRIVATE ssize_t tty_read(unsigned minor, char *buf, size_t n)
{
	char c;  /* Working character. */
	char *p; /* Write pointer.     */
	
	UNUSED(minor);
	
	p = buf;
	
	/* Read characters. */
	while (n > 0)
	{
		/* Wait for data. */
		if (sleep_empty())
			return (-1);
		
		/* Copy data from input buffer. */
		while ((n > 0) && (!KBUFFER_EMPTY(tty.input)))
		{
			KBUFFER_GET(tty.input, *p);
			
			n--;
			c = *p++;
			
			/* Done reading. */
			if ((c == '\n') && (tty.term.c_lflag & ICANON))
				return ((ssize_t)(p - buf));
		}
	}
	
	return ((ssize_t)(p - buf));
}

/*
 * Opens a tty device.
 */
PRIVATE int tty_open(unsigned minor)
{	
	/* Assign controlling terminal. */
	if ((IS_LEADER(curr_proc)) && (curr_proc->tty == NULL_DEV))
	{
		/* tty already assigned. */
		if (tty.pgrp != NULL)
			return (-EBUSY);
		
		curr_proc->tty = DEVID(TTY_MAJOR, minor, CHRDEV);
		tty.pgrp = curr_proc;
	}
	
	return (0);
}

/*
 * tty device driver interface.
 */
PRIVATE struct cdev tty_driver = {
	&tty_open,  /* open().  */
	&tty_read,  /* read().  */
	&tty_write, /* write(). */
	NULL        /* ioctl(). */
};

/*
 * Initializes the tty device driver.
 */
PUBLIC void tty_init(void)
{		
	kprintf("dev: initializing tty device driver");
	
	/* Initialize tty. */
	tty.pgrp = NULL;
	KBUFFER_INIT(tty.output);
	KBUFFER_INIT(tty.input);
	tty.term.c_lflag |= ICANON | ECHO;
	
	/* Initialize device drivers. */
	console_init();
	keyboard_init();
	
	/* Register charecter device. */
	if (cdev_register(TTY_MAJOR, &tty_driver))
		kpanic("failed to register tty device driver");
	
	/* Change kernel's output device. */
	chkout(DEVID(TTY_MAJOR, 0, CHRDEV));
}

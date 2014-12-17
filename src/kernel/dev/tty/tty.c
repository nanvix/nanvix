/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <dev/tty.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <errno.h>
#include <termios.h>
#include "tty.h"

/**
 * @brief TTY devices.
 */
PRIVATE struct tty tty;

/**
 * @brief Currently active TTY device.
 */
PRIVATE struct tty *active = &tty;

/**
 * @brief Handles a TTY interrupt.
 * 
 * @details Handles a TTY interrupt, putting the received character @p ch in the
 *          raw input buffer of the currently active TTY device. Addiitonally, 
 *          if echo is  enable for such device, @p ch is output to the terminal.
 * 
 * @param ch Received character.
 */
PUBLIC void tty_int(unsigned char ch)
{
	KBUFFER_PUT(active->rinput, ch);
	wakeup(&active->rinput.chain);

	/* Output echo character. */
	if (active->term.c_lflag & ECHO)
	{
		/* 
		 * Let backspace be handled 
		 * when the line is being parsed
		 */
		if (ch == '\b')
			return;
			
		console_put(ch, WHITE);
	}
}

/**
 * @brief Sleeps if the raw input buffer of a TTY device is empty.
 * 
 * @details Puts the calling process to sleep if the raw input buffer of the TTY
 *          device pointed to by @p ttyp is empty.
 * 
 * @param ttyp TTY device to sleep for.
 * 
 * @returns Upon sucessfull completion, zero is returned, meaning that the 
 *          input buffer of the target TTY device is no longer empty. If the 
 *          process has awaken because it has been interrupted by a signal, 
 *          non-zero is returned instead. In this later case, it is undefined
 *          whether the buffer is no longer empty.
 * 
 * @note @p ttyp must point to a valid TTY device.
 */
PRIVATE int sleep_empty(struct tty *ttyp)
{
	/* Sleep while input buffer is empty. */
	while (KBUFFER_EMPTY(ttyp->rinput))
	{
		sleep(&ttyp->rinput.chain, PRIO_TTY);
		
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
		/* Copy data to output tty buffer. */
		while ((n > 0) && (!KBUFFER_FULL(tty.output)))
		{
			KBUFFER_PUT(tty.output, *p);
			
			p++, n--;
		}
		
		/* Flushes tty output buffer. */
		disable_interrupts();
		console_write(&tty.output);
		enable_interrupts();
	}
		
	return ((ssize_t)(p - buf));
}

/**
 * @brief Reads data from a TTY device.
 * 
 * @details Reads @p n bytes data from the TTY device, which minor device number
 *          is @p minor, to the buffer pointed to by @p buf.
 * 
 * @returns The number of bytes actually read to the TTY device.
 */
PRIVATE ssize_t tty_read(unsigned minor, char *buf, size_t n)
{
	unsigned char ch; /* Working character. */
	unsigned char *p; /* Write pointer.     */
	
	UNUSED(minor);
	
	p = (unsigned char *)buf;
	
	/* Read characters. */
	disable_interrupts();
	while (n > 0)
	{
		/* Wait for data. */
		if (sleep_empty(&tty))
		{
			enable_interrupts();
			return (-1);
		}
		
		KBUFFER_GET(tty.rinput, ch);
		
		/* Check signals. */
		if (tty.term.c_lflag & ISIG)
		{
			/* Interrupt. */
			if (ch == tty.term.c_cc[VINTR])
				continue;
		}
		
		/* Canonical mode. */
		if (tty.term.c_lflag & ICANON)
		{
			/* Erase. */
			if (ch == '\b')
			{
				if (!KBUFFER_EMPTY(tty.cinput))
				{
					KBUFFER_TAKEOUT(tty.cinput);
					console_put(ch, WHITE);
				}
			}
			
			else
			{
				KBUFFER_PUT(tty.cinput, ch);
			
				/* Copy data to input buffer. */
				if ((ch == '\n') || (KBUFFER_FULL(tty.cinput)))
				{		
					/* Copy data from input buffer. */
					while ((n > 0) && (!KBUFFER_EMPTY(tty.cinput)))
					{
						KBUFFER_GET(tty.cinput, *p);
						
						n--;
						ch = *p++;
						
						/* Done reading. */
						if ((ch == '\n') && (tty.term.c_lflag & ICANON))
							goto out;
					}
				}
			}
		}
		
		/* Non canonical mode. */
		else
		{
			*p++ = ch;
			n--;
		}
	}

out:

	enable_interrupts();
	
	return ((ssize_t)((char *)p - buf));
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
 * Gets tty settings.
 */
PRIVATE int tty_gets(struct tty *tty, struct termios *termiosp)
{
	/* Invalid termios pointer. */	
	if (!chkmem(termiosp, sizeof(struct termios), MAY_WRITE))
		return (-EINVAL);
	
	kmemcpy(termiosp, &tty->term, sizeof(struct termios));
	
	return (0);
}

/*
 * Cleans the console.
 */
PRIVATE int tty_clear(struct tty *tty)
{
	UNUSED(tty);
	console_clear();
	return (0);
}

/*
 * Performs control operation on a tty device.
 */
PRIVATE int tty_ioctl(unsigned minor, unsigned cmd, unsigned arg)
{
	int ret;
	
	UNUSED(minor);
	
	/* Parse command. */
	switch (cmd)
	{
		/* Get tty settings. */
		case TTY_GETS:
			ret = tty_gets(&tty, (struct termios *)arg);
			break;
		
		/* Clear console. */
		case TTY_CLEAR:
			ret = tty_clear(&tty);
			break;
		
		/* Invalid operation. */
		default:
			ret = -EINVAL;
			break;
	}
	
	return (ret);
}

/*
 * tty device driver interface.
 */
PRIVATE struct cdev tty_driver = {
	&tty_open,  /* open().  */
	&tty_read,  /* read().  */
	&tty_write, /* write(). */
	&tty_ioctl  /* ioctl(). */
};

/**
 * @brief Initial control characters.
 */
tcflag_t init_c_cc[NCCS] = {
	0x00, /**< EOF character.   */
	0x00, /**< EOL character.   */
	0x7f, /**< ERASE character. */
	0x03, /**< INTR character.  */
	0x15, /**< KILL character.  */
	0x00, /**< MIN value.       */
	0x4e, /**< QUIT character.  */
	0x17, /**< START character. */
	0x13, /**< STOP character.  */
	0x19, /**< SUSP character.  */
	0x00  /**< TIME value.      */
};

/*
 * Initializes the tty device driver.
 */
PUBLIC void tty_init(void)
{		
	kprintf("dev: initializing tty device driver");
	
	/* Initialize tty. */
	tty.pgrp = curr_proc;
	KBUFFER_INIT(tty.output);
	KBUFFER_INIT(tty.cinput);
	KBUFFER_INIT(tty.rinput);
	tty.term.c_lflag = ICANON | ECHO | ISIG;
	for (unsigned i = 0; i < NCCS; i++)
		tty.term.c_cc[i] = init_c_cc[i];
	
	/* Initialize device drivers. */
	console_init();
	keyboard_init();
	
	/* Register charecter device. */
	if (cdev_register(TTY_MAJOR, &tty_driver))
		kpanic("failed to register tty device driver");
	
	/* Change kernel's output device. */
	chkout(DEVID(TTY_MAJOR, 0, CHRDEV));
}

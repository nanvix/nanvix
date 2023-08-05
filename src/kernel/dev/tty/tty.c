/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2016 Davidson Francis <davidsondfgl@gmail.com>
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

#include <dev/tty.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <nanvix/syscall.h>
#include <errno.h>
#include <termios.h>
#include <stropts.h>
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
 * @brief Sends a signal to process group.
 *
 * @details Sends the signal @p sig to the process group of the currently active
 *          TTY device.
 *
 * @param sig Signal to be sent.
 */
PRIVATE void tty_signal(int sig)
{
	for (struct process *p = FIRST_PROC; p <= LAST_PROC; p++)
	{
		/* Skip invalid processes. */
		if (!IS_VALID(p))
			continue;

		if (active->pgrp == p->pgrp)
			sndsig(p, sig);
	}
}

/**
 * @brief Handles a TTY interrupt.
 *
 * @details Handles a TTY interrupt, putting the received character @p ch in the
 *          raw input buffer of the currently active TTY device. Addiitonally,
 *          if echo is enable for such device, @p ch is output to the terminal.
 *
 * @param ch Received character.
 */
PUBLIC void tty_int(unsigned char ch)
{
	/* Output echo character. */
	if (active->term.c_lflag & ECHO)
	{
		/* Canonical mode. */
		if (active->term.c_lflag & ICANON)
		{
			/*
			 * Let these characters be handled
			 * when the line is being parsed.
			 */
			if ((ch == ERASE_CHAR(active->term)) ||
				(ch == KILL_CHAR(active->term)) ||
				(ch == EOL_CHAR(active->term)) ||
				(ch == EOF_CHAR(active->term)))
				goto out1;
		}

		/* Non-printable characters. */
		if ((ch < 32) && (ch != '\n') && (ch != '\t'))
		{
			console_put('^', WHITE);
			console_put(ch + 64, WHITE);
			console_put('\n', WHITE);
		}

		/* Any character. */
		else
			console_put(ch, WHITE);
	}

	/*
	 * Handle signals. Note that if a signal is
	 * received, no character is put in the raw
	 * input buffer. Otherwise, we could mess it up.
	 */
	if (active->term.c_lflag & ISIG)
	{
		/*
		 * Interrupt. Send signal to all
		 * process in the same group.
		 */
		if (ch == INTR_CHAR(active->term))
		{
			tty_signal(SIGINT);
			goto out0;
		}

		/* Stop. */
		else if (ch == STOP_CHAR(active->term))
		{
			active->flags |= TTY_STOPPED;
			return;
		}

		/* Start. */
		else if (ch == START_CHAR(active->term))
		{
			active->flags &= ~TTY_STOPPED;
			wakeup(&active->output.chain);
			return;
		}

		/* Suspend. */
		else if (ch == SUSP_CHAR(active->term))
		{
			tty_signal(SIGTSTP);
			goto out0;
		}

		/* Quit. */
		else if (ch == QUIT_CHAR(active->term))
		{
			tty_signal(SIGQUIT);
			goto out0;
		}
	}

out1:
	KBUFFER_PUT(active->rinput, ch);
out0:
	wakeup(&active->rinput.chain);
}

/**
 * @brief Sleeps if the raw input buffer of a TTY device is empty.
 *
 * @details Puts the calling process to sleep if the raw input buffer of the TTY
 *          device pointed to by @p ttyp is empty.
 *
 * @param ttyp TTY device to sleep for.
 *
 * @returns Upon sucessful completion, zero is returned, meaning that the
 *          input buffer of the target TTY device is no longer empty. If while
 *          sleeping, the process gets awaken due to the deliver of a signal,
 *          -#EINTR is returned instead. In this later case, it is undefined
 *          whether the buffer is no longer empty.
 *
 * @note @p ttyp must point to a valid TTY device.
 */
PRIVATE int tty_sleep_empty(struct tty *ttyp)
{
	/* Sleep while raw input buffer is empty. */
	while (KBUFFER_EMPTY(ttyp->rinput))
	{
		sleep(&ttyp->rinput.chain, PRIO_TTY);

		/* Awaken by signal. */
		if (issig() != SIGNULL)
			return (-EINTR);
	}

	return (0);
}

/**
 * @brief Sleeps if the output buffer of a TTY device is full.
 *
 * @details Puts the calling process to sleep if the output buffer of the TTY
 *          device pointed to by @p ttyp is full.
 *
 * @param ttyp TTY device to sleep for.
 *
 * @returns Upon successful completion, zero is returned, meaning that the
 *          output buffer of the target TTY device is no longer full. If while
 *          sleeping, the process gets awaken due to the deliver of a signal,
 *          -#EINTR is returned instead. In this later case, it is undefined
 *          whether the buffer is no longer full.
 */
PRIVATE int tty_sleep_full(struct tty *ttyp)
{
	/* Sleep while output buffer is full. */
	while (KBUFFER_FULL(ttyp->output))
	{
		sleep(&ttyp->output.chain, PRIO_TTY);

		/* Awaken by signal. */
		if (issig() != SIGNULL)
			return (-EINTR);

		/* Awaken by START character. */
		disable_interrupts();
		console_write(&ttyp->output);
		enable_interrupts();
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
		/*
		 * Wait for free slots
		 * in the output buffer.
		 */
		if (tty_sleep_full(&tty))
			return (-EINTR);

		/* Copy data to output tty buffer. */
		while ((n > 0) && (!KBUFFER_FULL(tty.output)))
		{
			KBUFFER_PUT(tty.output, *p);

			p++, n--;
		}

		/* Flushes tty output buffer. */
		if (!(tty.flags & TTY_STOPPED))
		{
			disable_interrupts();
			console_write(&tty.output);
			enable_interrupts();
		}
	}

	return ((ssize_t)(p - buf));
}

/**
 * @brief Reads data from a TTY device.
 *
 * @details Reads @p n bytes data from the TTY device, which minor device number
 *          is @p minor, to the buffer pointed to by @p buf.
 *
 * @param minor Minor device number of target TTY device.
 * @param buf   Buffer where data shall be placed.
 * @param n     Number of bytes to be read.
 *
 * @returns The number of bytes actually read to the TTY device.
 */
PRIVATE ssize_t tty_read(unsigned minor, char *buf, size_t n)
{
	size_t i;         /* # bytes read.      */
	unsigned char ch; /* Working character. */
	unsigned char *p; /* Write pointer.     */

	UNUSED(minor);

	i = n;
	p = (unsigned char *)buf;

	/* Read characters. */
	disable_interrupts();
	while (i > 0)
	{
		/* Canonical mode. */
		if (tty.term.c_lflag & ICANON)
		{
			/* Wait for data to become available. */
			if (tty_sleep_empty(&tty))
			{
				enable_interrupts();
				return (-EINTR);
			}

			KBUFFER_GET(tty.rinput, ch);

			/* Erase. */
			if (ch == ERASE_CHAR(tty.term))
			{
				if (!KBUFFER_EMPTY(tty.cinput))
				{
					KBUFFER_TAKEOUT(tty.cinput);
					console_put(ch, WHITE);
				}
			}

			/* Kill. */
			else if (ch == KILL_CHAR(tty.term))
			{
				while (!KBUFFER_EMPTY(tty.cinput))
				{
					i = n;
					p = (unsigned char *)buf;
					KBUFFER_TAKEOUT(tty.cinput);
					console_put('\b', WHITE);
				}
			}

			else
			{
				/* End of file. */
				if (ch == EOF_CHAR(tty.term))
					ch = '\0';

				/* End of line. */
				else if (ch == EOL_CHAR(tty.term))
					console_put(ch = '\n', WHITE);

				KBUFFER_PUT(tty.cinput, ch);

				/* Copy data to input buffer. */
				if ((ch == '\n') || (KBUFFER_FULL(tty.cinput)) || (ch == '\0'))
				{
					/* Copy data from input buffer. */
					while ((i > 0) && (!KBUFFER_EMPTY(tty.cinput)))
					{
						KBUFFER_GET(tty.cinput, ch);

						/* EOF. */
						if (ch == '\0')
							goto out;

						i--;
						*p++ = ch;

						/* Done reading. */
						if (ch == '\n')
							goto out;
					}
				}
			}
		}

		/* Non canonical mode. */
		else
		{
			if (MIN_CHAR(tty.term) > 0)
			{
				/* Case A: MIN>0, TIME>0 */
				if (TIME_CHAR(tty.term) > 0)
				{
					kprintf("tty: MIN>0, TIME>0");
					goto out;
				}

				/* Case B: MIN>0, TIME=0 */
				else
				{
					/* Wait for data to become available. */
					if (tty_sleep_empty(&tty))
					{
						enable_interrupts();
						return (-EINTR);
					}

					/* Copy data from input buffer. */
					while ((i > 0) && (!KBUFFER_EMPTY(tty.rinput)))
					{
						KBUFFER_GET(tty.rinput, ch);
						i--;
						*p++ = ch;
					}
				}
			}

			else
			{
				/* Case C: MIN=0, TIME>0 */
				if (TIME_CHAR(tty.term) > 0)
				{
					kprintf("tty: MIN=0, TIME>0");
					goto out;
				}

				/* Case D: MIN=0, TIME=0 */
				else
				{
					/* Done reading. */
					if (KBUFFER_EMPTY(tty.rinput))
						goto out;

					KBUFFER_GET(tty.rinput, ch);

					i--;
					*p++ = ch;
				}
			}
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
 * Sets tty settings
 */
PRIVATE int tty_sets(struct tty *tty, int options, struct termios *termiosp)
{
	int ret;

	ret = 0;

	/* Invalid termios pointer. */
	if (!chkmem(termiosp, sizeof(struct termios), MAY_READ))
		return (-EINVAL);

	/*
	 * For now, only TCSANOW is supported.
	 */
	switch (options)
	{
		/* The change occurs immediately. */
		case TCSANOW:
			kmemcpy(&tty->term, termiosp, sizeof(struct termios));
			break;

		/* Invalid operation. */
		default:
			ret = -EINVAL;
			break;
	}

	return (ret);
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
	switch (IOCTL_MAJOR(cmd))
	{
		/* Get tty settings. */
		case IOCTL_MAJOR(TTY_GETS):
			ret = tty_gets(&tty, (struct termios *)arg);
			break;

		/* Set tty settings */
		case IOCTL_MAJOR(TTY_SETS):
			ret = tty_sets(&tty, IOCTL_MINOR(cmd), (struct termios *)arg);
			break;

		/* Clear console. */
		case IOCTL_MAJOR(TTY_CLEAR):
			ret = tty_clear(&tty);
			break;

		/* Invalid operation. */
		default:
			ret = -EINVAL;
			break;
	}

	return (ret);
}

/**
 * @brief Closes a TTY device.
 *
 * @details Closes the TTY device with minor device number @p minor.
 *
 * @param minor Minor device number of target TTY device.
 *
 * @returns Zero is always returned.
 */
PRIVATE int tty_close(unsigned minor)
{
	UNUSED(minor);

	tty.pgrp = NULL;

	sys_kill(0, SIGHUP);

	return (0);
}

/*
 * tty device driver interface.
 */
PRIVATE struct cdev tty_driver = {
	&tty_open,  /* open().  */
	&tty_read,  /* read().  */
	&tty_write, /* write(). */
	&tty_ioctl, /* ioctl(). */
	&tty_close  /* ioctl(). */
};

/**
 * @brief Initial control characters.
 */
PRIVATE tcflag_t init_c_cc[NCCS] = {
	'd' - 96,  /**< EOF character.   */
	'\n',      /**< EOL character.   */
	'\b',      /**< ERASE character. */
	'c' - 96,  /**< INTR character.  */
	'u' - 96,  /**< KILL character.  */
	0x00,      /**< MIN value.       */
	'\\' - 64, /**< QUIT character.  */
	'q' - 96,  /**< START character. */
	's' - 96,  /**< STOP character.  */
	'z' - 96,  /**< SUSP character.  */
	0x00       /**< TIME value.      */
};

/*
 * Initializes the tty device driver.
 */
PUBLIC void tty_init(void)
{
	kprintf("dev: initializing tty device driver");

	/* Initialize tty. */
	tty.flags = 0;
	tty.pgrp = NULL;
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
}
